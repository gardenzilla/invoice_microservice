extern crate base64;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use chrono::{DateTime, NaiveDate, Utc};
use gzlib::proto::invoice::*;
use gzlib::proto::purchase::PaymentKind;
use invoice::PaymentMethod;
use packman::*;
use prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, error::Error};
use tokio::sync::{mpsc, oneshot, Mutex};
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

mod file;
mod invoice;
mod prelude;
mod szamlazzhu;

// How many worker can work together
// const WORKER_MAX: u32 = 2;

const PDF_FOLDER_NAME: &'static str = "pdf";

struct InvoiceProcessor<T>
where
  T: invoice::InvoiceAgent + Send,
{
  agent: Arc<Mutex<T>>,
}

impl<T> InvoiceProcessor<T>
where
  T: invoice::InvoiceAgent + Send,
{
  fn new(agent: T) -> Self
  where
    T: invoice::InvoiceAgent + Send,
  {
    InvoiceProcessor {
      agent: Arc::new(Mutex::new(agent)),
    }
  }
  async fn start(
    &mut self,
    mut new_invoice_chan_rx: mpsc::Receiver<invoice::InvoiceObject>,
    invoice_objects: Arc<Mutex<VecPack<invoice::InvoiceObject>>>,
    invoices: Arc<Mutex<VecPack<invoice::Invoice>>>,
  ) {
    // Do the processes
    // Infinite loop till the sender is alive
    while let Some(invoice_object) = new_invoice_chan_rx.recv().await {
      // Clone invoices
      let invoices = invoices.clone();

      // Clone invoice_objects
      let invoice_objects = invoice_objects.clone();

      let inner_id = invoice_object.internal_id;

      match self.agent.lock().await.create_invoice(invoice_object).await {
        Ok(invoice_summary) => {
          // Then try to save it as a PDF file
          match file::base64_decode(&invoice_summary.pdf_base64.replace("\n", "")) {
            Ok(bytes) => file::save_file(
              bytes,
              std::path::PathBuf::from(format!(
                "data/{}/{}.pdf",
                PDF_FOLDER_NAME, invoice_summary.invoice_id
              )),
            )
            .await
            .expect(&format!(
              "Invoice PDF SAVE ERROR: {}",
              invoice_summary.invoice_id
            )),
            Err(_) => error!(
              "Invoice PDF BASE64 DECODE error: {}",
              invoice_summary.invoice_id
            ),
          }

          // Set InvoiceID
          invoices
            .lock()
            .await
            .into_iter()
            .filter(|i| i.unpack().get_id() == &inner_id)
            .for_each(|i| {
              i.as_mut().unpack().invoice_id = Some(invoice_summary.invoice_id.clone())
            });

          // And remove InvoiceObject
          invoice_objects
            .lock()
            .await
            .remove_pack(&inner_id)
            .expect("Error while removing invoice object from storage");
        }
        Err(_) => {
          {
            // Set error occured
            invoices
              .lock()
              .await
              .into_iter()
              .filter(|i| i.unpack().get_id() == &inner_id)
              .for_each(|i| {
                i.as_mut().unpack().has_error = true;
              });

            // And remove InvoiceObject
            invoice_objects
              .lock()
              .await
              .remove_pack(&inner_id)
              .expect("Error while removing invoice object from storage");
          }
        }
      }
    }

    error!("Background process closed!");
  }
}

struct InvoiceService {
  send_channel: Mutex<mpsc::Sender<invoice::InvoiceObject>>,
  invoice_store: Arc<Mutex<VecPack<invoice::Invoice>>>,
  invoice_object_store: Arc<Mutex<VecPack<invoice::InvoiceObject>>>,
}

impl InvoiceService {
  fn new(
    sender: mpsc::Sender<invoice::InvoiceObject>,
    invoices: Arc<Mutex<VecPack<invoice::Invoice>>>,
    invoice_objects: Arc<Mutex<VecPack<invoice::InvoiceObject>>>,
  ) -> Self {
    Self {
      send_channel: Mutex::new(sender),
      invoice_store: invoices,
      invoice_object_store: invoice_objects,
    }
  }

  async fn create_new(&self, r: InvoiceForm) -> ServiceResult<InvoiceData> {
    // Create seller
    let seller = invoice::Seller::new();

    // Create customer
    let c = match r.customer {
      Some(_customer) => _customer,
      None => return Err(ServiceError::internal_error("Missing customer object")),
    };
    let customer = invoice::Customer::new(c.name, c.tax_number, c.zip, c.location, c.street);

    // Date parser helper
    // DateTime RFC3339 to NaiveDate
    let date_parser = |datestr: &str| -> ServiceResult<NaiveDate> {
      let date = DateTime::parse_from_rfc3339(datestr)
        .map_err(|_| ServiceError::internal_error("A megadott dátum hibás"))?
        .with_timezone(&Utc);
      Ok(date.naive_utc().date())
    };

    let payment_kind: PaymentKind = PaymentKind::from_i32(r.payment_kind)
      .ok_or(ServiceError::internal_error("Wrong paymentkind ENUM!"))?;

    let header = invoice::Header::new(
      date_parser(&r.date)?,
      date_parser(&r.completion_date)?,
      date_parser(&r.payment_duedate)?,
      match payment_kind {
        PaymentKind::Cash => PaymentMethod::Cash,
        PaymentKind::Card => PaymentMethod::Card,
        PaymentKind::Transfer => PaymentMethod::Transfer,
      },
    );

    let map_item = |i: &invoice_form::Item| -> ServiceResult<invoice::Item> {
      invoice::Item::new(
        i.name.to_string(),
        i.quantity,
        i.unit.to_string(),
        i.price_unit_net,
        invoice::VAT::from_str(&i.vat).map_err(|e| ServiceError::bad_request(&e))?,
        i.total_price_net,
        i.total_price_vat,
        i.total_price_gross,
      )
      .map_err(|_| {
        ServiceError::bad_request("A megadott tétel ár adatai (nettó, áfa, bruttó) nem helyesek!")
      })
    };

    let items = r
      .items
      .iter()
      .map(map_item)
      .collect::<Result<Vec<invoice::Item>, ServiceError>>()?;

    // Create Invoice Object
    let invoice_object = invoice::InvoiceObject::new(
      r.purchase_id,
      seller,
      customer,
      header,
      items,
      r.total_net,
      r.total_gross,
      r.total_vat,
      chrono::Utc::now(),
      r.created_by,
    );

    // Save invoice object to invoice_object_store
    self
      .invoice_object_store
      .lock()
      .await
      .insert(invoice_object.clone())
      .map_err(|e| {
        ServiceError::internal_error(&format!(
          "Error inserting new invoice object to iobject storage {}",
          e
        ))
      })?;

    // Send InvoiceObject to create
    self
      .send_channel
      .lock()
      .await
      .send(invoice_object.clone())
      .await
      .map_err(|_| {
        ServiceError::internal_error("Error while sending invoice_object via send_channel")
      })?;

    let i: invoice::Invoice = invoice_object.into();

    // Store invoice
    self
      .invoice_store
      .lock()
      .await
      .insert(i.clone())
      .map_err(|_| ServiceError::internal_error("Error while saving invoice to invoice store"))?;

    Ok(i.into())
  }

  async fn get_by_id(&self, r: ByIdRequest) -> ServiceResult<InvoiceData> {
    let id = Uuid::parse_str(&r.id).map_err(|_| ServiceError::bad_request("Hibás ID! Nem UUID"))?;
    let res = self
      .invoice_store
      .lock()
      .await
      .find_id(&id)?
      .unpack()
      .clone();

    Ok(res.into())
  }

  async fn download(&self, r: DownloadRequest) -> ServiceResult<DownloadResponse> {
    let pdf_base64 = file::load_invoice_base64(&r.invoice_id)
      .await
      .map_err(|e| ServiceError::internal_error(&e.to_string()))?;

    Ok(DownloadResponse { pdf_base64 })
  }
}

#[tonic::async_trait]
impl invoice_server::Invoice for InvoiceService {
  async fn create_new(
    &self,
    request: Request<InvoiceForm>,
  ) -> Result<Response<InvoiceData>, Status> {
    let res = self.create_new(request.into_inner()).await?;
    Ok(Response::new(res))
  }

  async fn get_by_id(
    &self,
    request: Request<ByIdRequest>,
  ) -> Result<Response<InvoiceData>, Status> {
    let res = self.get_by_id(request.into_inner()).await?;
    Ok(Response::new(res))
  }

  async fn download(
    &self,
    request: Request<DownloadRequest>,
  ) -> Result<Response<DownloadResponse>, Status> {
    let res = self.download(request.into_inner()).await?;
    Ok(Response::new(res))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  pretty_env_logger::init();

  info!("Server started!");

  // Create pdf folder path if not exist
  std::fs::create_dir_all(format!("data/{}", PDF_FOLDER_NAME))
    .expect("Error while creating PDF folder path");

  // Channels for new invoice requests
  let (mut new_invoice_sender, new_invoice_rx) = mpsc::channel::<invoice::InvoiceObject>(100);

  // Load Invoice Object Store (New invoice requests)
  let invoice_object_store: Arc<Mutex<VecPack<invoice::InvoiceObject>>> = Arc::new(Mutex::new(
    VecPack::load_or_init(PathBuf::from("data/invoice_objects"))
      .expect("Error loading invoice objects storage"),
  ));

  // Load Invoices storage (Done)
  let invoice_store: Arc<Mutex<VecPack<invoice::Invoice>>> = Arc::new(Mutex::new(
    VecPack::load_or_init(PathBuf::from("data/invoices")).expect("Error loading invoices storage"),
  ));

  let agent = szamlazzhu::SzamlazzHu::new();

  let invoice_object_store_clone = invoice_object_store.clone();
  let invoice_store_clone = invoice_store.clone();

  // Parallel thread for invoice processor
  tokio::task::spawn(async move {
    // Start invoice processor
    InvoiceProcessor::new(agent)
      .start(
        new_invoice_rx,
        invoice_object_store_clone,
        invoice_store_clone,
      )
      .await;
  });

  // Send unprocessed invoice objects to processor
  for invoice in invoice_object_store.lock().await.iter() {
    let _ = new_invoice_sender.send(invoice.unpack().clone()).await;
  }

  let addr = env::var("SERVICE_ADDR_INVOICE")
    .unwrap_or("[::1]:50060".into())
    .parse()
    .unwrap();

  // Create shutdown channel
  let (tx, rx) = oneshot::channel();

  let invoice_service = InvoiceService::new(
    new_invoice_sender.clone(),
    invoice_store.clone(),
    invoice_object_store.clone(),
  );

  // Spawn the server into a runtime
  tokio::task::spawn(async move {
    Server::builder()
      .add_service(invoice_server::InvoiceServer::new(invoice_service))
      .serve_with_shutdown(addr, async {
        let _ = rx.await;
      })
      .await
      .unwrap()
  });

  tokio::signal::ctrl_c().await?;

  println!("SIGINT");

  // Send shutdown signal after SIGINT received
  let _ = tx.send(());

  Ok(())
}
