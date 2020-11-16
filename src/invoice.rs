use chrono::{DateTime, NaiveDate, Utc};
use packman::VecPackMember;
use serde::{Deserialize, Serialize};
use std::ops::Mul;

pub trait InvoiceAgent {
    fn create_invoice(&self, data: InvoiceObject) -> Result<InvoiceSummary, AgentError>;
}

#[derive(Debug)]
pub enum AgentError {
    DataError(String),
    // ServiceError,
    InternalError(String),
}

#[derive(Debug)]
pub struct InvoiceSummary {
    pub invoice_id: String,
    pub pdf_base64: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Invoice {
    pub id: u32,
    pub purchase_id: u32,
    pub invoice_id: Option<String>,
    pub related_storno: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    // status: Status,
}

impl Default for Invoice {
    fn default() -> Self {
        Invoice {
            id: 0,
            purchase_id: 0,
            invoice_id: None,
            related_storno: None,
            created_by: String::default(),
            created_at: Utc::now(),
        }
    }
}

impl VecPackMember for Invoice {
    type Out = u32;

    fn get_id(&self) -> &Self::Out {
        &self.id
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InvoiceObject {
    pub internal_id: u32,
    pub external_id: Option<String>,
    pub cart_id: u32,
    pub seller: Seller,
    pub customer: Customer,
    pub header: Header,
    pub items: Vec<Item>,
    pub total_net: i32,
    pub total_gross: i32,
    pub total_vat: i32,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

impl InvoiceObject {
    pub fn new(
        internal_id: u32,
        cart_id: u32,
        seller: Seller,
        customer: Customer,
        header: Header,
        items: Vec<Item>,
        total_net: i32,
        total_gross: i32,
        total_vat: i32,
        created_at: DateTime<Utc>,
        created_by: String,
    ) -> Self {
        InvoiceObject {
            internal_id,
            external_id: None,
            cart_id,
            seller,
            customer,
            header,
            items,
            total_net,
            total_gross,
            total_vat,
            created_at,
            created_by,
        }
    }
}

impl Default for InvoiceObject {
    fn default() -> Self {
        InvoiceObject {
            internal_id: 0,
            external_id: None,
            cart_id: 0,
            seller: Seller::default(),
            customer: Customer::default(),
            header: Header::default(),
            items: Vec::new(),
            total_net: 0,
            total_gross: 0,
            total_vat: 0,
            created_at: Utc::now(),
            created_by: "".into(),
        }
    }
}

impl From<InvoiceObject> for Invoice {
    fn from(i: InvoiceObject) -> Self {
        Invoice {
            id: i.internal_id,
            purchase_id: i.cart_id,
            invoice_id: i.external_id,
            related_storno: None,
            created_by: i.created_by,
            created_at: i.created_at,
        }
    }
}

impl VecPackMember for InvoiceObject {
    type Out = u32;
    fn get_id(&self) -> &Self::Out {
        &self.internal_id
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Seller {}

impl Seller {
    pub fn new() -> Self {
        Seller {}
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Customer {
    pub name: String,
    pub tax_number: String,
    pub zip: String,
    pub location: String,
    pub street: String,
}

impl Customer {
    pub fn new(
        name: String,
        tax_number: String,
        zip: String,
        location: String,
        street: String,
    ) -> Self {
        Customer {
            name,
            tax_number,
            zip,
            location,
            street,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PaymentMethod {
    Cash,
    Transfer,
    Card,
}

impl Default for PaymentMethod {
    fn default() -> Self {
        PaymentMethod::Cash
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Header {
    pub date_created: String,
    pub date_completion: String,
    pub payment_duedate: String,
    pub payment_method: PaymentMethod,
}

impl Header {
    pub fn new(
        date_created: NaiveDate,
        date_completion: NaiveDate,
        payment_duedate: NaiveDate,
        payment_method: PaymentMethod,
    ) -> Self {
        Header {
            date_created: date_created.to_string(),
            date_completion: date_completion.to_string(),
            payment_duedate: payment_duedate.to_string(),
            payment_method: payment_method,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Item {
    pub name: String,
    pub quantity: i32,
    pub unit: String,
    pub retail_price_net: i32,
    pub vat: VAT,
    pub total_price_net: i32,
    pub total_price_vat: i32,
    pub total_price_gross: i32,
}

#[derive(Debug)]
pub enum ItemError {
    TotalUnitNetError,
    TotalUnitGrossError,
}

impl ToString for ItemError {
    fn to_string(&self) -> String {
        match self {
            ItemError::TotalUnitNetError => "Nem megfelelő az adott tétel totál nettó ára!".into(),
            ItemError::TotalUnitGrossError => {
                "Nem megfelelő az adott tétel totál bruttó ára!".into()
            }
        }
    }
}

impl Item {
    pub fn new(
        name: String,
        quantity: i32,
        unit: String,
        retail_price_net: i32,
        vat: VAT,
        total_price_net: i32,
        total_price_vat: i32,
        total_price_gross: i32,
    ) -> Result<Self, ItemError> {
        if (quantity * retail_price_net) != total_price_net {
            return Err(ItemError::TotalUnitNetError);
        }
        if (quantity * retail_price_net * vat.clone()) != total_price_gross {
            return Err(ItemError::TotalUnitGrossError);
        }
        if (total_price_net + total_price_vat) != total_price_gross {
            return Err(ItemError::TotalUnitGrossError);
        }
        Ok(Self {
            name,
            quantity,
            unit,
            retail_price_net,
            vat,
            total_price_net,
            total_price_vat,
            total_price_gross,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VAT {
    AAM,
    FAD,
    TAM,
    _5,
    _18,
    _27,
}

impl Default for VAT {
    fn default() -> Self {
        VAT::_27
    }
}

impl VAT {
    pub fn from_str(str: &str) -> Result<VAT, String> {
        match str {
            "AAM" => Ok(VAT::AAM),
            "aam" => Ok(VAT::AAM),
            "FAD" => Ok(VAT::FAD),
            "fad" => Ok(VAT::FAD),
            "TAM" => Ok(VAT::TAM),
            "tam" => Ok(VAT::TAM),
            "5" => Ok(VAT::_5),
            "18" => Ok(VAT::_18),
            "27" => Ok(VAT::_27),
            _ => Err("Nem megfelelő Áfa formátum! 5, 18, 27, AAM, TAM, FAD".into()),
        }
    }
}

impl Mul<VAT> for i32 {
    type Output = i32;

    fn mul(self, rhs: VAT) -> Self::Output {
        let res = match rhs {
            VAT::AAM => self as f32 * 1.0,
            VAT::FAD => self as f32 * 1.0,
            VAT::TAM => self as f32 * 1.0,
            VAT::_5 => self as f32 * 1.05,
            VAT::_18 => self as f32 * 1.18,
            VAT::_27 => self as f32 * 1.27,
        };
        res.round() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_vat_multiply() {
        use VAT::*;
        assert_eq!(100, 100 * AAM);
        assert_eq!(100, 100 * FAD);
        assert_eq!(100, 100 * TAM);
        assert_eq!(118, 100 * _18);
        assert_eq!(105, 100 * _5);
        assert_eq!(127, 100 * _27);
        assert_eq!(1415, 1114 * _27);
        assert_eq!((1114 * _27) * 9, 1114 * _27 * 9);
    }
}
