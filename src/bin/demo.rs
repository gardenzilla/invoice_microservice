fn calculate_check(number: &[u32]) -> u32 {
    let mut number = number.iter().map(|c| c.clone()).collect::<Vec<u32>>();
    number.reverse();
    let check = number
        .iter()
        .enumerate()
        .map(|(position, number)| {
            // let i = 10_usize.pow(position as u32) as u32;
            // let n = (number % i) / (i / 10);
            number * (position as u32 + 1)
        })
        .fold(0, |acc, x| acc + x);
    match 10 - (check % 10) {
        x if x == 10 => 0,
        x => x,
    }
}

fn str_to_vec_u32(to: &str) -> Vec<u32> {
    to.to_string()
        .chars()
        .into_iter()
        .map(|c| c.to_digit(10).unwrap())
        .collect::<Vec<u32>>()
}

pub struct UplId();

impl UplId {
    pub fn from_u32(from: u32) -> u32 {
        let vu32 = str_to_vec_u32(&from.to_string());
        let check = calculate_check(&vu32);
        format!(
            "{}{}{}",
            match check {
                x if x == 0 => 9,
                x => 10 - x,
            },
            from,
            check
        )
        .parse::<u32>()
        .unwrap()
    }
}

fn u32_to_vec_u32(to: u32) -> Vec<u32> {
    str_to_vec_u32(&to.to_string())
}

fn is_valid(number: &str) -> bool {
    let v = str_to_vec_u32(number);
    let check = calculate_check(&v[0..v.len() - 1]);
    v.last().unwrap() == &check
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("-----------------");
    let mut res: Vec<u32> = vec![];
    for i in 1..100 as u32 {
        let r = UplId::from_u32(i);
        res.push(r);
    }
    res.sort();
    for item in res {
        println!("{}", item);
        println!("Is valid {}", is_valid(&format!("{}", item)));
    }
    Ok(())
}
