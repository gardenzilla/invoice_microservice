extern crate reqwest;

mod invoice;
mod szamlazzhu;

use chrono::prelude::*;
use futures::executor;
use quick_xml::de::from_str;
use std::error;

use szamlazzhu::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let settings = Settings::new(Some(
        std::env::var("INVOICE_AGENT_KEY").expect("NO AGENT KEY ENV!"),
    ));
    let seller = Seller::new();
    let customer = Customer::new(
        "Demo Customer".to_string(),
        "4400".to_string(),
        "Demo city".to_string(),
        "Demo street".to_string(),
        None,
    );
    let waybill = Waybill::new();
    let header = Header::new(
        Utc::today().naive_local(),
        Utc::today().naive_local(),
        Utc::today().naive_local(),
        PaymentMethod::Cash,
        None,
        "GRDN".to_string(),
    );
    let item_a = Item::new(
        "DemoA".to_string(),
        1,
        "db".to_string(),
        100.0,
        27,
        100.0,
        27.0,
        127.0,
        None,
    );
    let item_b = Item::new(
        "DemoB".to_string(),
        1,
        "db".to_string(),
        100.0,
        27,
        100.0,
        27.0,
        127.0,
        None,
    );
    let invoice_request = InvoiceRequest::new(
        settings,
        header,
        seller,
        customer,
        waybill,
        vec![item_a, item_b],
    );
    let r = invoice_request.unwrap();
    let client = reqwest::Client::new();
    // println!("Query is\n\n {}", &r);
    let body = client
        .post("https://www.szamlazz.hu/szamla/")
        .form(&[("action-xmlagentxmlfile", r.as_str())])
        .send()
        .await?;
    let text = executor::block_on(body.text()).unwrap();
    println!("Response is\n\n {}", &text);
    let response: SzamlazzHuResponse = from_str(text.trim()).unwrap();
    println!("{:?}", response);
    Ok(())
}
