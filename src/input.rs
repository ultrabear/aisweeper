use std::io::{stdin, stdout, Error, Write};

pub fn input_fn(s: &str) -> Result<String, Error> {
	let mut o = stdout().lock();
	let i = stdin();

	o.write(s.as_bytes())?;
	o.flush()?;

	let mut out = String::new();
	i.read_line(&mut out)?;

	out.pop();

	Ok(out)
}

#[macro_export]
macro_rules! input {
  () => {

    crate::input::input_fn("")
  };

  ($($arg:tt)*) => {
    crate::input::input_fn(&(format!($($arg)*)))
  };
}
