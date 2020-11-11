pub struct Invoice {
    internal_id: u32,
    external_id: String,
    seller: Seller,
    customer: Customer,
    header: Header,
    items: Vec<Item>,
    total_net: (),
    total_gross: (),
}

pub struct Seller {}
pub struct Customer {}
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
