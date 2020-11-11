extern crate reqwest;

use chrono::prelude::*;
use futures::executor;
use quick_xml::de::DeError;
use quick_xml::se::to_string;
use serde::Serialize;
use std::error;
use tokio::prelude::*;

struct InvoiceRequest {}

impl InvoiceRequest {
    pub fn new(
        settings: Settings,
        header: Header,
        seller: Seller,
        customer: Customer,
        waybill: Waybill,
        items: Vec<Item>,
    ) -> Result<String, DeError> {
        let s = serialize(&settings)?;
        let h = serialize(&header)?;
        let s2 = serialize(&seller)?;
        let c = serialize(&customer)?;
        // let w = serialize(&waybill)?;
        let i = serialize(&items)?;
        let intro = r#"<xmlszamla xmlns="http://www.szamlazz.hu/xmlszamla" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.szamlazz.hu/xmlszamla https://www.szamlazz.hu/szamla/docs/xsds/agent/xmlszamla.xsd">"#;
        Ok(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}{}{}{}{}<tetelek>{}</tetelek></xmlszamla>",
            intro, s, h, s2, c, i
        ))
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "beallitasok")]
struct Settings {
    #[serde(rename = "felhasznalo")]
    username: Option<String>,
    #[serde(rename = "jelszo")]
    password: Option<String>,
    #[serde(rename = "szamlaagentkulcs")]
    agent_key: Option<String>,
    #[serde(rename = "eszamla")]
    is_einvoice: bool,
    #[serde(rename = "szamlaLetoltes")]
    should_download_pdf: bool,
    #[serde(rename = "valaszVerzio")]
    response_version: u32,
    #[serde(rename = "aggregator")]
    agregator: Option<String>,
}

impl Settings {
    pub fn new(agent_key: Option<String>) -> Self {
        Settings {
            username: None,
            password: None,
            agent_key,
            is_einvoice: false,
            should_download_pdf: true,
            response_version: 2,
            agregator: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "fejlec")]
struct Header {
    #[serde(rename = "keltDatum", with = "date_format")]
    date_created: NaiveDate,
    #[serde(rename = "teljesitesDatum", with = "date_format")]
    completion_date: NaiveDate,
    #[serde(rename = "fizetesiHataridoDatum", with = "date_format")]
    payment_duedate: NaiveDate,
    #[serde(rename = "fizmod")]
    payment_method: String,
    #[serde(rename = "penznem")]
    currency: String,
    #[serde(rename = "szamlaNyelve")]
    language: String,
    #[serde(rename = "megjegyzes")]
    comment: Option<String>,
    #[serde(rename = "arfolyamBank")]
    exchange_rate_bank: String,
    #[serde(rename = "arfolyam")]
    exchange_rate: f32,
    #[serde(rename = "rendelesSzam")]
    order_number: Option<String>,
    #[serde(rename = "dijbekeroSzamlaszam")]
    proforma_id: Option<String>,
    #[serde(rename = "elolegszamla")]
    is_deposit_invoice: bool,
    #[serde(rename = "vegszamla")]
    is_final_invoice: bool,
    #[serde(rename = "helyesbitoszamla")]
    is_corrective_invoice: bool,
    #[serde(rename = "helyesbitettSzamlaszam")]
    corrected_invoce_id: Option<String>,
    #[serde(rename = "dijbekero")]
    is_proform_invoice: bool,
    #[serde(rename = "szamlaszamElotag")]
    invoice_prefix: String,
}

mod date_format {
    use chrono::NaiveDate;
    use serde::{self, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }
}

pub enum PaymentMethod {
    Cash,
    CreditCar,
    Transfer,
}

impl PaymentMethod {
    fn to_string(&self) -> String {
        match self {
            PaymentMethod::Cash => String::from("Készpénz"),
            PaymentMethod::CreditCar => String::from("Bankkártya"),
            PaymentMethod::Transfer => String::from("Átutalás"),
        }
    }
}

pub enum Currency {
    Huf,
}

impl Currency {
    fn to_string(&self) -> String {
        match self {
            Currency::Huf => String::from("HUF"),
        }
    }
}

pub enum Language {
    Hu,
}

impl Language {
    fn to_string(&self) -> String {
        match self {
            Language::Hu => String::from("hu"),
        }
    }
}

impl Header {
    pub fn new(
        date_created: NaiveDate,
        completion_date: NaiveDate,
        payment_duedate: NaiveDate,
        payment_method: PaymentMethod,
        comment: Option<String>,
        invoice_prefix: String,
    ) -> Self {
        Header {
            date_created,
            completion_date,
            payment_duedate,
            payment_method: payment_method.to_string(),
            currency: Currency::Huf.to_string(),
            language: Language::Hu.to_string(),
            comment,
            exchange_rate_bank: "MNB".to_string(),
            exchange_rate: 0.0,
            order_number: None,
            proforma_id: None,
            is_deposit_invoice: false,
            is_final_invoice: false,
            is_corrective_invoice: false,
            corrected_invoce_id: None,
            is_proform_invoice: false,
            invoice_prefix,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "elado")]
struct Seller {
    #[serde(rename = "bank")]
    bank: String,
    #[serde(rename = "bankszamlaszam")]
    bank_account: String,
    #[serde(rename = "emailReplyto")]
    email_reply_to: Option<String>,
    #[serde(rename = "emailTargy")]
    email_subject: Option<String>,
    #[serde(rename = "emailSzoveg")]
    email_body: Option<String>,
}

impl Seller {
    pub fn new() -> Self {
        Seller {
            bank: "K&H Bank".to_string(),
            bank_account: "10404436-50526580-66701009".to_string(),
            email_reply_to: None,
            email_subject: None,
            email_body: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "vevo")]
struct Customer {
    #[serde(rename = "nev")]
    name: String,
    #[serde(rename = "irsz")]
    zip: String,
    #[serde(rename = "telepules")]
    location: String,
    #[serde(rename = "cim")]
    address: String,
    #[serde(rename = "email")]
    email: Option<String>,
    #[serde(rename = "sendEmail")]
    should_send_email: bool,
    #[serde(rename = "adoszam")]
    taxnumber: Option<String>,
    #[serde(rename = "postazasiNev")]
    post_name: Option<String>,
    #[serde(rename = "postazasiIrsz")]
    post_zip: Option<String>,
    #[serde(rename = "postazasiTelepules")]
    post_location: Option<String>,
    #[serde(rename = "postazasiCim")]
    post_address: Option<String>,
    #[serde(rename = "azonosito")]
    id: Option<String>,
    #[serde(rename = "telefonszam")]
    phone: Option<String>,
    #[serde(rename = "megjegyzes")]
    comment: Option<String>,
}

impl Customer {
    pub fn new(
        name: String,
        zip: String,
        location: String,
        address: String,
        taxnumber: Option<String>,
    ) -> Self {
        Customer {
            name,
            zip,
            location,
            address,
            email: None,
            should_send_email: false,
            taxnumber,
            post_name: None,
            post_zip: None,
            post_location: None,
            post_address: None,
            id: None,
            phone: None,
            comment: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "fuvarlevel")]
struct Waybill {
    #[serde(rename = "uticel")]
    address: Option<String>,
    #[serde(rename = "futarszolgalat")]
    courier_service: Option<String>,
}

impl Waybill {
    pub fn new() -> Self {
        Waybill {
            address: None,
            courier_service: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "tetel")]
struct Item {
    #[serde(rename = "megnevezes")]
    name: String,
    #[serde(rename = "mennyiseg")]
    quantity: u32,
    #[serde(rename = "mennyisegiEgyseg")]
    unit: String,
    #[serde(rename = "nettoEgysegar")]
    net_retail_price: f32,
    #[serde(rename = "afakulcs")]
    vat_percentage: u32,
    #[serde(rename = "nettoErtek")]
    total_net_price: f32,
    #[serde(rename = "afaErtek")]
    total_vat: f32,
    #[serde(rename = "bruttoErtek")]
    total_gross_price: f32,
    #[serde(rename = "megjegyzes")]
    comment: Option<String>,
}

impl Item {
    pub fn new(
        name: String,
        quantity: u32,
        unit: String,
        net_retail_price: f32,
        vat_percentage: u32,
        total_net_price: f32,
        total_vat: f32,
        total_gross_price: f32,
        comment: Option<String>,
    ) -> Self {
        Item {
            name,
            quantity,
            unit,
            net_retail_price,
            vat_percentage,
            total_net_price,
            total_vat,
            total_gross_price,
            comment,
        }
    }
}

fn serialize<T>(invoice_request: &T) -> Result<String, DeError>
where
    T: Serialize,
{
    to_string(invoice_request)
}

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
    println!("Response is\n\n {}", text);
    Ok(())
}
