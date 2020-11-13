extern crate base64;

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
            // Invoice Objects insert new invoice
            if let Ok(mut invoice_objects) = invoice_objects.lock() {
                let _ = invoice_objects.insert(new_invoice.clone());
            }

            // InvoiceObject into() Invoice, save it to invoices
            if let Ok(mut invoice_storage) = invoices.lock() {
                let _ = invoice_storage.insert(new_invoice.clone().into());
            }

            // Create invoice by agent
            let summary = self.agent.create_invoice(new_invoice);
            if let Ok(summary) = summary {
                // Update invoice with invoice_id + set success status
                summary.invoice_id;
                ();
                // Save Base64 as PDF file
                summary.pdf_base64;
                ();
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let invoice_object = invoice::InvoiceObject {
        internal_id: 1,
        external_id: None,
        cart_id: 1,
        seller: invoice::Seller::new(),
        customer: invoice::Customer::new(
            "Demo Elek".into(),
            "".into(),
            "4551".into(),
            "Nyíregyháza".into(),
            "Mogyorós utca 36.".into(),
        ),
        header: invoice::Header::new(
            NaiveDate::from_ymd(2020, 11, 13),
            NaiveDate::from_ymd(2020, 11, 13),
            NaiveDate::from_ymd(2020, 11, 13),
            invoice::PaymentMethod::Transfer,
        ),
        items: vec![invoice::Item::new(
            "Demo item".into(),
            1,
            "db".into(),
            100,
            invoice::VAT::_27,
            100,
            27,
            127,
        )
        .unwrap()],
        total_net: 100,
        total_gross: 127,
        created_at: chrono::Utc::now(),
        created_by: "mezeipetister".into(),
    };
    let agent = szamlazzhu::SzamlazzHu::new();
    let res = agent.create_invoice(invoice_object);
    let _res = &res.unwrap();
    let bytes = file::base64_decode(&_res.pdf_base64.replace("\n", "")).unwrap();
    file::save_file(
        bytes,
        std::path::PathBuf::from(format!("pdf/{}.pdf", _res.invoice_id)),
    )
    .unwrap();
}

// fn main() {
//     // Channels for new invoice requests
//     let (new_invoice_sender, new_invoice_rx) = mpsc::channel::<invoice::InvoiceObject>();

//     // Load Invoice Object Store (New invoice requests)
//     let invoice_object_store: Arc<Mutex<VecPack<invoice::InvoiceObject>>> = Arc::new(Mutex::new(
//         VecPack::load_or_init(PathBuf::from("data/invoice_objects"))
//             .expect("Error loading invoice objects storage"),
//     ));

//     // Load Invoices storage (Done)
//     let invoice_store: Arc<Mutex<VecPack<invoice::Invoice>>> = Arc::new(Mutex::new(
//         VecPack::load_or_init(PathBuf::from("data/invoices"))
//             .expect("Error loading invoices storage"),
//     ));

//     let agent = szamlazzhu::SzamlazzHu::new();

//     // Parallel thread for invoice processor
//     std::thread::spawn(move || {
//         // Start invoice processor
//         InvoiceProcessor::new(agent).start(
//             new_invoice_rx,
//             invoice_object_store.clone(),
//             invoice_store.clone(),
//         )
//     });
// }
