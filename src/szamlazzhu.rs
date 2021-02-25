use std::env::temp_dir;

use quick_xml::de::{from_str, DeError};
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};

pub struct SzamlazzHu {
  agent_key: String,
  invoice_prefix: String,
  bank_name: String,
  bank_account: String,
}

impl SzamlazzHu {
  pub fn new() -> Self {
    SzamlazzHu {
      // Set szamlazz.hu agent key from ENV variable
      agent_key: std::env::var("INVOICE_AGENT_KEY")
        .expect("Cannot create SzamlazzHu Agent. NO AGENT KEY ENV!"),
      // Set szamlazz.hu agent key from ENV variable
      invoice_prefix: std::env::var("INVOICE_PREFIX")
        .expect("Cannot create SzamlazzHu Agent. NO INVOICE_PREFIX ENV!"),
      // Set szamlazz.hu agent key from ENV variable
      bank_name: std::env::var("INVOICE_BANK_NAME")
        .expect("Cannot create SzamlazzHu Agent. NO INVOICE_BANK_NAME ENV!"),
      // Set szamlazz.hu agent key from ENV variable
      bank_account: std::env::var("INVOICE_BANK_ACCOUNT")
        .expect("Cannot create SzamlazzHu Agent. NO INVOICE_BANK_ACCOUNT ENV!"),
    }
  }
}

impl From<crate::invoice::VAT> for VAT {
  fn from(v: crate::invoice::VAT) -> Self {
    match v {
      crate::invoice::VAT::AAM => VAT::AAM,
      crate::invoice::VAT::FAD => VAT::FAD,
      crate::invoice::VAT::TAM => VAT::TAM,
      crate::invoice::VAT::_5 => VAT::_5,
      crate::invoice::VAT::_18 => VAT::_18,
      crate::invoice::VAT::_27 => VAT::_27,
    }
  }
}

impl From<crate::invoice::PaymentMethod> for PaymentMethod {
  fn from(m: crate::invoice::PaymentMethod) -> Self {
    match m {
      crate::invoice::PaymentMethod::Cash => PaymentMethod::Cash,
      crate::invoice::PaymentMethod::Transfer => PaymentMethod::Transfer,
      crate::invoice::PaymentMethod::Card => PaymentMethod::CreditCar,
    }
  }
}

#[tonic::async_trait]
impl crate::invoice::InvoiceAgent for SzamlazzHu {
  async fn create_invoice(
    &self,
    data: crate::invoice::InvoiceObject,
  ) -> Result<crate::invoice::InvoiceSummary, crate::invoice::AgentError> {
    // Create settings object
    let settings = Settings::new(Some(self.agent_key.clone()));

    // Create seller object
    let seller = Seller::new(self.bank_name.clone(), self.bank_account.clone());

    // Create customer object
    let customer = Customer::new(
      data.customer.name,
      data.customer.zip,
      data.customer.location,
      data.customer.street,
      if data.customer.tax_number.len() > 0 {
        Some(data.customer.tax_number)
      } else {
        None
      },
    );
    let waybill = Waybill::new();
    let header = Header::new(
      data.header.date_created,
      data.header.date_completion,
      data.header.payment_duedate,
      PaymentMethod::Cash,
      None,
      self.invoice_prefix.clone(),
    );

    // Create item(s) vector
    let items = data
      .items
      .iter()
      .map(|i| {
        Item::new(
          i.name.to_string(),
          i.quantity,
          i.unit.to_string(),
          i.retail_price_net,
          i.vat.clone().into(),
          i.total_price_net,
          i.total_price_vat,
          i.total_price_gross,
          None,
        )
      })
      .collect::<Vec<Item>>();

    let invoice_request = InvoiceRequest::new(settings, header, seller, customer, waybill, items);
    let r = invoice_request.unwrap();
    let client = reqwest::Client::new();

    let response = client
      .post("https://www.szamlazz.hu/szamla/")
      .form(&[("action-xmlagentxmlfile", r.as_str())])
      .send()
      .await
      .map_err(|e| crate::invoice::AgentError::DataError(e.to_string()))?;

    let text = response
      .text()
      .await
      .map_err(|e| crate::invoice::AgentError::DataError(e.to_string()))?;

    let response: SzamlazzHuResponse = from_str(text.trim())
      .map_err(|e| crate::invoice::AgentError::InternalError(e.to_string()))?;

    Ok(response.into())
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "xmlszamlavalasz")]
pub struct SzamlazzHuResponse {
  #[serde(rename = "sikeres")]
  successfull: bool,
  #[serde(rename = "szamlaszam")]
  invoice_id: String,
  #[serde(rename = "szamlanetto")]
  total_net: f32,
  #[serde(rename = "szamlabrutto")]
  total_gross: f32,
  #[serde(rename = "kintlevoseg")]
  outstanding: f32,
  // TODO! Error while deserialize this field. Probably should escape it?
  // #[serde(rename = "vevoifiokurl")]
  // invoice_url: String,
  #[serde(rename = "pdf")]
  pdf_blob_base64: String,
}

impl From<SzamlazzHuResponse> for crate::invoice::InvoiceSummary {
  fn from(r: SzamlazzHuResponse) -> Self {
    crate::invoice::InvoiceSummary {
      invoice_id: r.invoice_id,
      pdf_base64: r.pdf_blob_base64,
      has_error: !r.successfull,
    }
  }
}
pub struct InvoiceRequest {}

impl InvoiceRequest {
  pub fn new(
    settings: Settings,
    header: Header,
    seller: Seller,
    customer: Customer,
    _waybill: Waybill,
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
pub struct Settings {
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
  #[serde(rename = "szamlaLetoltesPld")]
  copy_of_pdf_pages: u32,
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
      copy_of_pdf_pages: 1,
      response_version: 2,
      agregator: None,
    }
  }
}

#[derive(Debug, Serialize)]
#[serde(rename = "fejlec")]
pub struct Header {
  #[serde(rename = "keltDatum")]
  date_created: String,
  #[serde(rename = "teljesitesDatum")]
  completion_date: String,
  #[serde(rename = "fizetesiHataridoDatum")]
  payment_duedate: String,
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
  #[serde(rename = "szamlaSablon")]
  template: String,
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
    date_created: String,
    completion_date: String,
    payment_duedate: String,
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
      template: "Szla8cm".to_string(),
    }
  }
}

#[derive(Debug, Serialize)]
#[serde(rename = "elado")]
pub struct Seller {
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
  pub fn new(bank: String, bank_account: String) -> Self {
    Seller {
      bank,
      bank_account,
      email_reply_to: None,
      email_subject: None,
      email_body: None,
    }
  }
}

#[derive(Debug, Serialize)]
#[serde(rename = "vevo")]
pub struct Customer {
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
pub struct Waybill {
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
pub struct Item {
  #[serde(rename = "megnevezes")]
  name: String,
  #[serde(rename = "mennyiseg")]
  quantity: i32,
  #[serde(rename = "mennyisegiEgyseg")]
  unit: String,
  #[serde(rename = "nettoEgysegar")]
  net_retail_price: i32,
  #[serde(rename = "afakulcs")]
  vat: String,
  #[serde(rename = "nettoErtek")]
  total_net_price: i32,
  #[serde(rename = "afaErtek")]
  total_vat: i32,
  #[serde(rename = "bruttoErtek")]
  total_gross_price: i32,
  #[serde(rename = "megjegyzes")]
  comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VAT {
  AAM,
  TAM,
  FAD,
  _0,
  _5,
  _18,
  _27,
}

impl ToString for VAT {
  fn to_string(&self) -> String {
    use VAT::*;
    match self {
      AAM => "AAM".into(),
      TAM => "TAM".into(),
      FAD => "F.AFA".into(),
      _0 => "0".into(),
      _5 => "5".into(),
      _18 => "18".into(),
      _27 => "27".into(),
    }
  }
}

impl Item {
  pub fn new(
    name: String,
    quantity: i32,
    unit: String,
    net_retail_price: i32,
    vat: VAT,
    total_net_price: i32,
    total_vat: i32,
    total_gross_price: i32,
    comment: Option<String>,
  ) -> Self {
    Item {
      name,
      quantity,
      unit,
      net_retail_price,
      vat: vat.to_string(),
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
