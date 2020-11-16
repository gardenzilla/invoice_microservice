extern crate base64;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod file;
mod invoice;
mod szamlazzhu;

use chrono::NaiveDate;
use gzlib::proto;
use packman::*;
use proto::invoice::*;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tonic::{transport::Server, Request, Response, Status};

// use tokio::sync::Mutex;

// How many worker can work together
// const WORKER_MAX: u32 = 2;

const PDF_FOLDER_NAME: &'static str = "pdf";

struct InvoiceProcessor<T>
where
    T: invoice::InvoiceAgent,
{
    agent: T,
}

impl<T> InvoiceProcessor<T>
where
    T: invoice::InvoiceAgent,
{
    fn new(agent: T) -> Self
    where
        T: invoice::InvoiceAgent,
    {
        InvoiceProcessor { agent }
    }
    fn start(
        &mut self,
        new_invoice_chan_rx: mpsc::Receiver<invoice::InvoiceObject>,
        invoice_objects: Arc<Mutex<VecPack<invoice::InvoiceObject>>>,
        invoices: Arc<Mutex<VecPack<invoice::Invoice>>>,
    ) {
        // Do the processes
        // Infinite loop till the sender is alive
        for new_invoice in new_invoice_chan_rx {
            let inner_id = new_invoice.internal_id;
            // Create invoice by agent
            let summary = self.agent.create_invoice(new_invoice);
            if let Ok(summary) = summary {
                // Update invoice with invoice_id + set success status
                // summary.invoice_id;
                match invoices.lock() {
                    Ok(mut invoice_storage) => invoice_storage
                        .into_iter()
                        .filter(|i| i.unpack().get_id() == &inner_id)
                        .for_each(|i| {
                            i.as_mut().unpack().invoice_id = Some(summary.invoice_id.clone())
                        }),
                    Err(_) => error!("Error while locking invoice storage"),
                }

                // Remove InvoiceObject
                match invoice_objects.lock() {
                    Ok(mut iobjects) => match iobjects.remove_pack(&inner_id) {
                        Ok(_) => (),
                        Err(_) => error!("Couldnt remove InvoiceObject {}", &inner_id),
                    },
                    Err(_) => error!("Error while locking invoice objects storage"),
                }
                // Save Base64 as PDF file
                match file::base64_decode(&summary.pdf_base64.replace("\n", "")) {
                    Ok(bytes) => {
                        match file::save_file(
                            bytes,
                            std::path::PathBuf::from(format!(
                                "data/{}/{}.pdf",
                                PDF_FOLDER_NAME, summary.invoice_id
                            )),
                        ) {
                            Ok(_) => (),
                            Err(_) => error!("Invoice PDF SAVE ERROR: {}", summary.invoice_id),
                        }
                    }
                    Err(_) => error!("Invoice PDF BASE64 DECODE error: {}", summary.invoice_id),
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
}

#[tonic::async_trait]
impl invoice_server::Invoice for InvoiceService {
    async fn create_new(
        &self,
        request: Request<InvoiceForm>,
    ) -> Result<Response<NewInvoiceResponse>, Status> {
        let r = request.into_inner();
        let next_id = match self.invoice_store.lock() {
            Ok(invoices) => match invoices.last() {
                Some(last) => last.unpack().id + 1,
                None => 1,
            },
            Err(_) => panic!("Error while locking invoices"),
        };
        let seller = invoice::Seller::new();
        let c = match r.customer {
            Some(_customer) => _customer,
            None => return Err(Status::invalid_argument("No customer object found")),
        };
        let customer = invoice::Customer::new(c.name, c.tax_number, c.zip, c.location, c.street);

        let naive_date_parser = |datestr: &str| -> Result<NaiveDate, Status> {
            match NaiveDate::parse_from_str(datestr, "%Y-%m-%d") {
                Ok(d) => Ok(d),
                Err(_) => Err(Status::invalid_argument(
                    "A megadott dátum nem megfelelő formátumú: YYYY-MM-DD",
                )),
            }
        };

        let header = invoice::Header::new(
            naive_date_parser(&r.date)?,
            naive_date_parser(&r.completion_date)?,
            naive_date_parser(&r.payment_duedate)?,
            match r.payment_kind.as_str() {
                "cash" => invoice::PaymentMethod::Cash,
                "transfer" => invoice::PaymentMethod::Transfer,
                "card" => invoice::PaymentMethod::Card,
                _ => {
                    return Err(Status::invalid_argument(
                        "A megadott fizetési módszer nem megfelelő: card | cash | transfer",
                    ))
                }
            },
        );

        let purchase_id = match u32::from_str_radix(&r.purchase_id, 16) {
            Ok(pid) => pid,
            Err(_) => {
                return Err(Status::invalid_argument(
                    "A megadott kosár ID nem megfelelő! RADIX16 error",
                ))
            }
        };

        let map_item = |i: &invoice_form::Item| -> Result<invoice::Item, Status> {
            invoice::Item::new(
                i.name.to_string(),
                i.quantity,
                i.unit.to_string(),
                i.price_unit_net,
                invoice::VAT::from_str(&i.vat).map_err(|e| Status::invalid_argument(e))?,
                i.total_price_net,
                i.total_price_vat,
                i.total_price_gross,
            )
            .map_err(|_| {
                Status::invalid_argument(
                    "A megadott tétel ár adatai (nettó, áfa, bruttó) nem helyesek!",
                )
            })
        };

        let items = r
            .items
            .iter()
            .map(map_item)
            .collect::<Result<Vec<invoice::Item>, Status>>()?;

        let invoice_object = invoice::InvoiceObject::new(
            next_id,
            purchase_id,
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

        self.send_channel
            .lock()
            .map_err(|_| Status::internal("Error while locking send_channel"))?
            .send(invoice_object.clone())
            .map_err(|_| Status::internal("Error while sending invoice_object via send_channel"))?;

        // Save invoice object to invoice_object_store
        match self.invoice_object_store.lock() {
            Ok(mut iobject_store) => {
                iobject_store.insert(invoice_object.clone()).map_err(|e| {
                    Status::internal(format!(
                        "Error inserting new invoice object to iobject storage {}",
                        e
                    ))
                })?;
            }
            Err(_) => return Err(Status::internal("Error while locking object_store")),
        }

        let i: invoice::Invoice = invoice_object.into();

        match self.invoice_store.lock() {
            Ok(mut invoice_store) => {
                invoice_store
                    .insert(i.clone())
                    .map_err(|_| Status::internal("Error while saving invoice to invoice store"))?;
            }
            Err(_) => return Err(Status::internal("Error while locking invoice_store")),
        }

        Ok(Response::new(NewInvoiceResponse {
            invoice: Some(InvoiceData {
                id: format!("{:x}", i.id),
                purchase_id: format!("{:x}", i.purchase_id),
                invoice_id: match i.invoice_id {
                    Some(iid) => iid,
                    None => "".into(),
                },
                related_storno: match i.related_storno {
                    Some(s) => s,
                    None => "".into(),
                },
                created_by: i.created_by,
                created_at: i.created_at.clone().to_rfc3339(),
            }),
        }))
    }

    async fn get_by_id(
        &self,
        request: Request<ByIdRequest>,
    ) -> Result<Response<ByIdResponse>, Status> {
        let id = match u32::from_str_radix(&request.into_inner().id, 16) {
            Ok(id) => id,
            Err(_) => return Err(Status::invalid_argument("Bad invoice ID!")),
        };
        let i = self
            .invoice_store
            .lock()
            .map_err(|_| Status::internal("Error while locking invoice store"))?
            .find_id(&id)
            .map_err(|_| Status::not_found("A megadott számla azonosító nem található"))?
            .unpack()
            .clone();

        let u32_to_string = |u| format!("{:x}", u);

        Ok(Response::new(ByIdResponse {
            invoice: Some(InvoiceData {
                id: u32_to_string(i.id),
                purchase_id: u32_to_string(i.purchase_id),
                invoice_id: i.invoice_id.unwrap_or("".into()),
                related_storno: i.related_storno.unwrap_or("".into()),
                created_by: i.created_by,
                created_at: i.created_at.to_rfc3339(),
            }),
        }))
    }

    async fn get_by_purchase_id(
        &self,
        request: Request<PurchaseIdBulkRequest>,
    ) -> Result<Response<PurchaseIdBulkResponse>, Status> {
        let id = u32::from_str_radix(&request.into_inner().purchase_id, 16)
            .map_err(|_| Status::invalid_argument("A megadott ID nem megfelelő formátumú"))?;

        let invoices = self
            .invoice_store
            .lock()
            .map_err(|_| Status::internal("Error locking invoice store"))?
            .iter()
            .filter(|i| i.unpack().purchase_id == id)
            .map(|i| i.unpack().clone())
            .collect::<Vec<invoice::Invoice>>();

        let mut result: Vec<InvoiceData> = Vec::new();

        let u32_to_string = |u| format!("{:x}", u);

        for i in invoices {
            result.push(InvoiceData {
                id: u32_to_string(i.id),
                purchase_id: u32_to_string(i.purchase_id),
                invoice_id: i.invoice_id.unwrap_or("".into()),
                related_storno: i.related_storno.unwrap_or("".into()),
                created_by: i.created_by,
                created_at: i.created_at.to_rfc3339(),
            });
        }

        Ok(Response::new(PurchaseIdBulkResponse { invoices: result }))
    }

    async fn download(
        &self,
        request: Request<DownloadRequest>,
    ) -> Result<Response<DownloadResponse>, Status> {
        let pdf_base64 = file::load_invoice_base64(&request.into_inner().invoice_id)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DownloadResponse {
            pdf_base64: pdf_base64,
        }))
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
    let (new_invoice_sender, new_invoice_rx) = mpsc::channel::<invoice::InvoiceObject>();

    // Load Invoice Object Store (New invoice requests)
    let invoice_object_store: Arc<Mutex<VecPack<invoice::InvoiceObject>>> = Arc::new(Mutex::new(
        VecPack::load_or_init(PathBuf::from("data/invoice_objects"))
            .expect("Error loading invoice objects storage"),
    ));

    // Load Invoices storage (Done)
    let invoice_store: Arc<Mutex<VecPack<invoice::Invoice>>> = Arc::new(Mutex::new(
        VecPack::load_or_init(PathBuf::from("data/invoices"))
            .expect("Error loading invoices storage"),
    ));

    let agent = szamlazzhu::SzamlazzHu::new();

    let invoice_object_store_clone = invoice_object_store.clone();
    let invoice_store_clone = invoice_store.clone();

    // Parallel thread for invoice processor
    tokio::task::spawn_blocking(move || {
        // Start invoice processor
        InvoiceProcessor::new(agent).start(
            new_invoice_rx,
            invoice_object_store_clone,
            invoice_store_clone,
        );
    })
    .await?;

    // Send unprocessed invoice objects to processor
    match invoice_object_store.lock() {
        Ok(iobjectstore) => iobjectstore.iter().for_each(|i| {
            let _ = new_invoice_sender.send(i.unpack().clone());
        }),
        Err(_) => error!("Error while locking invoice_object_store!"),
    }

    let addr = std::env::var("SERVICE_ADDR_INVOICE")
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
