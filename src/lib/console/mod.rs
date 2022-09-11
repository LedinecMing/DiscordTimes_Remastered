use std::io::{
    prelude::*,
    stdin, stdout,
    Stdout
};


pub fn pause() {
    let mut stdin = stdin();
    let mut stdout = stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    write!(stdout, "Press any key to continue...").unwrap();
    stdout.flush().unwrap();

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
}

pub fn input(your_io: &mut Stdout, input_string: &str) -> String {
    const START: usize = 0;
    your_io.write(input_string.as_ref()).unwrap();
    your_io.flush().unwrap();
    let mut input = "".to_string();
    stdin().read_line(&mut input).unwrap();
    let end = input.chars().count() - 1;
    input.chars().take(end).skip(START).collect()
}
