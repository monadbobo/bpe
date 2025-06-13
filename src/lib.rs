use std::collections::{BTreeSet, HashMap};

pub fn compress(data: &[u8], max_freq: i8) -> (Vec<u8>, HashMap<u8, (u8, u8)>) {
    let mut empty_slot = BTreeSet::new();
    for i in 0..=255 {
        empty_slot.insert(i);
    }
    for d in data {
        if empty_slot.contains(d) {
            empty_slot.remove(d);
        }
    }

    let mut frequency_map: HashMap<(u8, u8), u32> = HashMap::new();
    let mut compressed_data = Vec::new();
    for i in 0..data.len() {
        if i + 1 < data.len() {
            let key = (data[i], data[i + 1]);
            *frequency_map.entry(key).or_insert(0) += 1;
        }
        compressed_data.push(data[i]);
    }
    let mut table = HashMap::new();
    loop {
        let mut found = false;
        for i in 0..compressed_data.len() {
            if i + 1 >= compressed_data.len() {
                break;
            }
            if frequency_map[&(compressed_data[i], compressed_data[i + 1])] >= max_freq as u32 {
                if table.contains_key(&(compressed_data[i], compressed_data[i + 1])) {
                    continue;
                }
                let e = empty_slot.pop_first().unwrap();
                table.insert((compressed_data[i], compressed_data[i + 1]), e);
                compressed_data[i] = e;
                frequency_map.remove(&(compressed_data[i], compressed_data[i + 1]));
                compressed_data.remove(i + 1);
                found = true;
            }
        }

        // must re init frequency_map
        frequency_map.clear();
        for i in 0..compressed_data.len() {
            if i + 1 < compressed_data.len() {
                let key = (compressed_data[i], compressed_data[i + 1]);
                *frequency_map.entry(key).or_insert(0) += 1;
            }
        }
        if !found {
            break;
        }
    }

    (
        compressed_data,
        table
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect::<HashMap<_, _>>(),
    )
}

pub fn decompress(compressed_data: &[u8], table: &HashMap<u8, (u8, u8)>) -> Vec<u8> {
    let mut decompressed_data = Vec::new();
    let mut i = 0;
    let mut stack = Vec::new();
    loop {
        let current_byte;

        if !stack.is_empty() {
            current_byte = stack.pop().unwrap();
        } else if i < compressed_data.len() {
            current_byte = compressed_data[i];
            i += 1;
        } else {
            break;
        }

        if let Some(&(a, b)) = table.get(&current_byte) {
            stack.push(b);
            stack.push(a);
        } else {
            decompressed_data.push(current_byte);
        }
    }
    decompressed_data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() {
        let data = vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 5];
        let max_freq = 2;
        let (compressed_data, table) = compress(&data, max_freq);
        let decompressed_data = decompress(&compressed_data, &table);
        assert_eq!(data, decompressed_data);
    }

    #[test]
    fn test_string_compress_decompress() {
        let data = b"hello world hello world";
        let max_freq = 2;
        let (compressed_data, table) = compress(data, max_freq);
        print!("Compressed data: {:?}\n", compressed_data);
        println!(
            "compress ratio: {}",
            compressed_data.len() as f64 / data.len() as f64
        );
        let decompressed_data = decompress(&compressed_data, &table);
        assert_eq!(data.to_vec(), decompressed_data);
    }
}
