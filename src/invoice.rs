use chrono::*;
use std::ops::Mul;

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
    retail_price_net: i32,
    vat: VAT,
    total_price_net: i32,
    total_price_vat: i32,
    total_price_gross: i32,
}

pub enum VAT {
    AAM,
    FAD,
    TAM,
    _5,
    _18,
    _27,
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
        assert_eq!(105, 100 * _5);
        assert_eq!(118, 100 * _18);
        assert_eq!(127, 100 * _27);
    }
}
