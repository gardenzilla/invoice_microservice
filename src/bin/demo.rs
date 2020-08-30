fn make_check(number: &mut [u32]) -> u32 {
    let mut number = number.clone();
    number.reverse();
    let check = number
        .iter()
        .enumerate()
        .map(|(position, number)| {
            let i = 10_usize.pow(position as u32) as u32;
            // let n = (number % i) / (i / 10);
            number * i
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

fn u32_to_vec_u32(to: u32) -> Vec<u32> {
    str_to_vec_u32(&to.to_string())
}

fn is_valid(number: &str) -> bool {
    let v = str_to_vec_u32(number);
    let check = make_check(&v[0..v.len() - 1]);
    v.last().unwrap() == &check
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("-----------------");
    let mut res: Vec<u32> = vec![];
    for i in 1..1000 {
        let r = format!("{}{}", i, make_check(&u32_to_vec_u32(i)))
            .parse::<u32>()
            .unwrap();
        res.push(r);
    }
    res.sort();
    for item in res {
        println!("{}", item);
        println!("Is valid {}", is_valid(&format!("{}", item)));
    }
    Ok(())
}
