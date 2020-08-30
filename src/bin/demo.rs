fn make_check(number: &str) -> usize {
    let len = number.len();
    let number = number.parse::<usize>().expect("Cannot parse to u32");
    let mut check = 0;
    for position in 1..len + 1 {
        let i = 10_usize.pow(position as u32);
        let n = (number % i) / (i / 10);
        check += n * position;
    }
    match 10 - (check % 10) {
        x if x == 10 => 0,
        x => x,
    }
}

fn is_valid(number: &str) -> bool {
    let check = make_check(&number[0..number.len() - 1]);
    let a = &number[number.len() - 1..number.len()];
    let b = check.to_string();
    a == b
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let number = "491";
    let with_check = format!("{}{}", number, make_check(number));
    println!("With check {}", with_check);
    println!("is valid {}", is_valid(&with_check));
    println!("-----------------");
    for i in 1..1000 {
        println!("{}", format!("{}{}", i, make_check(i.to_string().as_str())));
    }
    Ok(())
}
