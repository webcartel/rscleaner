pub fn plural_en(count: u64) -> usize {
    if count == 1 { 0 } else { 1 }
}

pub fn plural_ru(count: u64) -> usize {
    let modulo_10 = count % 10;
    let modulo_100 = count % 100;
    if modulo_10 == 1 && modulo_100 != 11 {
        0
    } else if (2..=4).contains(&modulo_10) && !(12..=14).contains(&modulo_100) {
        1
    } else {
        2
    }
}
