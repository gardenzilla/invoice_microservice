extern crate base64;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod file;
mod invoice;
mod szamlazzhu;

use crate::invoice::InvoiceAgent;
use chrono::NaiveDate;
use packman::*;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
// use tokio::sync::Mutex;

// How many worker can work together
// const WORKER_MAX: u32 = 2;

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
                            std::path::PathBuf::from(format!("pdf/{}.pdf", summary.invoice_id)),
                        ) {
                            Ok(_) => (),
                            Err(_) => error!("Invoice PDF SAVE ERROR: {}", summary.invoice_id),
                        }
                    }
                    Err(_) => error!("Invoice PDF BASE64 DECODE error: {}", summary.invoice_id),
                }
            }
        }
    }
}

// #[tokio::main]
// async fn main() {
//     let invoice_object = invoice::InvoiceObject {
//         internal_id: 1,
//         external_id: None,
//         cart_id: 1,
//         seller: invoice::Seller::new(),
//         customer: invoice::Customer::new(
//             "Demo Elek".into(),
//             "".into(),
//             "4551".into(),
//             "Nyíregyháza".into(),
//             "Mogyorós utca 36.".into(),
//         ),
//         header: invoice::Header::new(
//             NaiveDate::from_ymd(2020, 11, 13),
//             NaiveDate::from_ymd(2020, 11, 13),
//             NaiveDate::from_ymd(2020, 11, 13),
//             invoice::PaymentMethod::Transfer,
//         ),
//         items: vec![
//             invoice::Item::new(
//                 "Demo item".into(),
//                 1,
//                 "db".into(),
//                 100,
//                 invoice::VAT::_27,
//                 100,
//                 27,
//                 127,
//             )
//             .unwrap(),
//             invoice::Item::new(
//                 "Demo item 2".into(),
//                 3,
//                 "db".into(),
//                 1000,
//                 invoice::VAT::FAD,
//                 3000,
//                 0,
//                 3000,
//             )
//             .unwrap(),
//         ],
//         total_net: 3100,
//         total_gross: 3127,
//         created_at: chrono::Utc::now(),
//         created_by: "mezeipetister".into(),
//     };
//     let agent = szamlazzhu::SzamlazzHu::new();
//     let res = agent.create_invoice(invoice_object);
//     let _res = &res.unwrap();
//     let bytes = file::base64_decode(&_res.pdf_base64.replace("\n", "")).unwrap();
//     file::save_file(
//         bytes,
//         std::path::PathBuf::from(format!("pdf/{}.pdf", _res.invoice_id)),
//     )
//     .unwrap();
// }

fn main() {
    pretty_env_logger::init();

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

    // Parallel thread for invoice processor
    let invoice_object_store_clone = invoice_object_store.clone();
    std::thread::spawn(move || {
        // Start invoice processor
        InvoiceProcessor::new(agent).start(
            new_invoice_rx,
            invoice_object_store_clone,
            invoice_store.clone(),
        )
    });

    // Send unprocessed invoice objects to processor
    match invoice_object_store.lock() {
        Ok(iobjectstore) => iobjectstore.iter().for_each(|i| {
            let _ = new_invoice_sender.send(i.unpack().clone());
        }),
        Err(_) => error!("Error while locking invoice_object_store!"),
    }

    loop {}
}
