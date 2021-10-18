use crossbeam_channel;
use std::{thread, time};

struct Data<T: Send> {
    data: T,
    index: usize
}

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    // TODO: implement parallel map!
    let (input_tx, input_rx) = crossbeam_channel::unbounded::<Data<T>>();
    let (output_tx, output_rx) = crossbeam_channel::unbounded::<Data<U>>();
    let mut threads = Vec::new();

    for _ in 0..num_threads {
        let input_rx = input_rx.clone();
        let output_tx = output_tx.clone();
        threads.push(
            thread::spawn(move || {
                while let Ok(received) = input_rx.recv() {
                    let output: Data<U> = Data { data: f(received.data), index: received.index };
                    output_tx.send(output).unwrap();
                }
                drop(output_tx);
            })
        );
    }

    drop(output_tx);

    for (index, data) in input_vec.into_iter().enumerate() {
        input_tx.send(Data { data, index }).unwrap();
    }

    drop(input_tx);
    
    while let Ok(received) = output_rx.recv() {
        if output_vec.len() <= received.index {
            let len = output_vec.len();
            for _ in 0..(received.index - len + 1) {
                output_vec.push(U::default());
            }
        }
        output_vec[received.index] = received.data;
    }

    for handle in threads {
        handle.join().expect("Panic occurs in a thread!");
    }

    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
