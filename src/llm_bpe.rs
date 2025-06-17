use regex::Regex;
use std::collections::HashMap;

pub struct BpeCore {
    pub ranks: HashMap<Vec<u8>, u32>,
    pub encoder: Vec<Vec<u8>>,
    pub vocab_size: usize,
    pub pat_str: String,
    pub regex: Regex,
}

impl BpeCore {
    pub fn new(vocab_size: usize, pat_str: String) -> Self {
        assert!(
            vocab_size >= std::mem::size_of::<u8>(),
            "Data length exceeds 256 bytes"
        );
        let ranks = HashMap::new();
        let encoder = Vec::new();
        let regex = Regex::new(&pat_str).expect("Invalid regex pattern");
        BpeCore {
            ranks,
            encoder,
            vocab_size,
            pat_str,
            regex,
        }
    }

    pub fn train(&mut self, data: Vec<u8>) {
        assert!(
            self.vocab_size >= std::mem::size_of::<u8>(),
            "Data length exceeds 256 bytes"
        );
        for i in 0..256 {
            self.ranks.insert(vec![i as u8], i);
            self.encoder.push(vec![i as u8]);
        }

        // Convert Vec<u8> to String for regex processing
        let data_str = String::from_utf8_lossy(&data);

        // Splinter up our data into lists of bytes
        // data = "Hello world"
        // words = [
        //     [b'H', b'e', b'l', b'l', b'o'],
        //     [b' ', b'w', b'o', b'r', b'l', b'd']
        // ]
        let mut words: Vec<Vec<u32>> = self
            .regex
            .find_iter(&data_str)
            .map(|mat| mat.as_str().bytes().map(|b| b as u32).collect::<Vec<u32>>())
            .collect();

        while self.ranks.len() < self.vocab_size {
            println!(
                "Current vocabulary size: {}, target: {}",
                self.ranks.len(),
                self.vocab_size
            );
            let mut stat = HashMap::new();
            for p in words.iter() {
                for pair in p.windows(2) {
                    let key = (pair[0], pair[1]);
                    *stat.entry(key).or_insert(0) += 1;
                }
            }

            if stat.is_empty() {
                println!("No more pairs found, exiting training loop.");
                break; // No pairs found, exit the loop
            }

            let most_common_pair = stat
                .iter()
                .max_by_key(|&(_, &count)| count)
                .map(|(&pair, _)| pair)
                .unwrap();
            let byte1 = self.encoder[most_common_pair.0 as usize].clone();
            let byte2 = self.encoder[most_common_pair.1 as usize].clone();
            let new_token = [byte1, byte2].concat();
            let token = self.ranks.len();
            println!("Merging tokens: {:?}, {:?}", new_token, token);
            self.ranks.insert(new_token.clone(), token as u32);
            self.encoder.push(new_token);

            let mut new_words = Vec::new();

            for word in words.iter() {
                let mut new_word = Vec::new();
                let mut i = 0;
                while i < word.len() - 1 {
                    if (word[i], word[i + 1]) == most_common_pair {
                        new_word.push(token as u32);
                        i += 2; // Skip the next character since it's part of the pair
                    } else {
                        new_word.push(word[i]);
                        i += 1;
                    }
                }

                if i == word.len() - 1 {
                    new_word.push(word[i]);
                }
                new_words.push(new_word);
            }
            words = new_words;
        }
    }
}

fn get_rank(ranks: &HashMap<Vec<u8>, u32>, data: &[u8], parts: &[(usize, u32)], i: usize) -> u32 {
    if i + 3 >= parts.len() {
        return u32::MAX;
    }

    ranks
        .get(&data[parts[i].0..parts[i + 3].0])
        .cloned()
        .unwrap_or(u32::MAX)
}

fn merge(data: &[u8], ranks: &HashMap<Vec<u8>, u32>) -> Vec<(usize, u32)> {
    let mut parts = Vec::new();

    let mut min_rank = (u32::MAX, usize::MAX);
    for i in 0..data.len() - 1 {
        let pair = vec![data[i], data[i + 1]];
        let rank = if let Some(&rank) = ranks.get(&pair) {
            if rank < min_rank.0 {
                min_rank = (rank, i);
            }
            rank
        } else {
            u32::MAX
        };
        parts.push((i, rank));
    }
    parts.push((data.len() - 1, u32::MAX));
    parts.push((data.len(), u32::MAX));

    while min_rank.0 != u32::MAX {
        let (_, i) = min_rank;
        if i > 0 {
            parts[i - 1].1 = get_rank(ranks, &data, &parts, i - 1);
        }

        parts[i].1 = get_rank(ranks, &data, &parts, i);
        parts.remove(i + 1);
        min_rank = (u32::MAX, usize::MAX);
        for j in 0..parts.len() - 1 {
            if parts[j].1 < min_rank.0 {
                min_rank = (parts[j].1, j);
            }
        }
    }
    parts
}

pub fn pair_encode(data: &[u8], ranks: &HashMap<Vec<u8>, u32>) -> Vec<u32> {
    merge(data, ranks)
        .windows(2)
        .map(|part| ranks[&data[part[0].0..part[1].0]])
        .collect()
}

pub fn pair_split<'a>(data: &'a [u8], ranks: &HashMap<Vec<u8>, u32>) -> Vec<&'a [u8]> {
    merge(data, ranks)
        .windows(2)
        .map(|part| &data[part[0].0..part[1].0])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pair_split() {
        let data = b"Hello world".to_vec();
        let vocab_size = 1000;
        let pat_str = r#"'s|'t|'re|'ve|'m|'ll|'d| ?[\p{L}]+| ?[\p{N}]+| ?[^\s\p{L}\p{N}]+|\s+"#;
        let mut bpe = BpeCore::new(vocab_size, pat_str.to_string());
        bpe.train(data.clone());
        let encode = pair_encode(&data, &bpe.ranks);
        for e in encode.iter() {
            println!("Encoded: {}", e);
            let d = bpe.encoder[*e as usize].clone();
            println!("Decoded: {:?}", d);
        }
        let decoder: Vec<Vec<u8>> = encode
            .iter()
            .map(|&i| bpe.encoder[i as usize].clone())
            .collect();
        assert_eq!(
            decoder,
            vec![
                vec![b'H', b'e', b'l', b'l', b'o'],
                vec![b' ', b'w', b'o', b'r', b'l', b'd']
            ]
        );
    }
}
