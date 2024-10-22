use std::time;

struct XOR64 {
    x: u64,
}

impl XOR64 {
    fn new(seed: u64) -> Self {
        XOR64 {
            x: seed ^ 88172645463325252,
        }
    }

    fn next(&mut self) -> u64 {
        let x = self.x;
        let x = x ^ (x << 13);
        let x = x ^ (x >> 7);
        let x = x ^ (x << 17);
        self.x = x;
        x
    }
}

impl Iterator for XOR64 {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next())
    }
}

const N: usize = 20000000;

fn randomize_vec() -> (Vec<u64>, Vec<u64>) {
    let mut generator = XOR64::new(4321);
    let v1 = (&mut generator).take(N).collect::<Vec<_>>();
    let v2 = (&mut generator).take(N).collect::<Vec<_>>();

    (v1, v2)
}

fn single_thread() {
    let (mut v1, mut v2) = randomize_vec();
    let start = time::Instant::now();

    v1.sort();
    v2.sort();

    let end = start.elapsed();

    println!("single thread: {}.{}s", end.as_secs(), end.subsec_micros())
}

fn dual_thread() {
    let (mut v1, mut v2) = randomize_vec();
    let start = time::Instant::now();

    let handler1 = std::thread::spawn(move || {
        v1.sort();
        v1
    });
    let handler2 = std::thread::spawn(move || {
        v2.sort();
        v2
    });

    handler1.join().unwrap();
    handler2.join().unwrap();

    let end = start.elapsed();

    println!("dual thread: {}.{}s", end.as_secs(), end.subsec_micros())
}

fn main() {
    single_thread();
    dual_thread();
}
