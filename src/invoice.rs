use chrono::*;

pub struct Invoice {
    id: u32,
    purchase_id: u32,
    related_storno: Option<String>,
    created_by: String,
    created_at: DateTime<Utc>,
    status: Status,
}

pub enum Status {
    Err { error_msg: String },
    Loading,
    Done { invoice_id: String },
}

pub struct InvoiceData {
    internal_id: u32,
    external_id: Option<String>,
    cart_id: u32,
    seller: Seller,
    customer: Customer,
    header: Header,
    items: Vec<Item>,
    total_net: f32,
    total_gross: f32,
}

pub struct Seller {}
pub struct Customer {
    name: String,
    zip: String,
}
pub struct Header {}
pub struct Item {
    name: String,
    retail_price_net: f32,
    vat: VAT,
    total_price_net: f32,
    total_price_vat: f32,
    total_price_gross: f32,
}

pub enum VAT {
    AAM,
    FAD,
    TAM,
    _0,
    _5,
    _27,
}
