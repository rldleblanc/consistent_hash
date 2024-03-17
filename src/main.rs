use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};
use substring::Substring;

const DISKS: u32 = 12;
const WEIGHT: u32 = 5;
// DISK * WEIGHT * VEC_MULT = Vector size
const VEC_MULT: u32 = 20;
const NUM_FILES: u32 = 1000;

#[derive(Debug)]
struct ConsistentHash {
    hashmap: Vec<String>,
    tries: i32,
    count: i32,
    finalized: bool,
    size: usize,
}

impl ConsistentHash {
    pub fn new(num: u32) -> ConsistentHash {
        println!("Creating vector with size: {}", num);
        ConsistentHash {
            hashmap: vec!["".to_string(); num as usize],
            tries: 0,
            count: 0,
            finalized: false,
            size: num as usize,
        }
    }
    pub fn add(&mut self, path: String) {
        //println!("INFO: Adding {}.", &path);
        assert!(
            !self.finalized,
            "Can't add to a finalized set, drop and recreate."
        );
        let hash = hash_32(&path);
        let mut idx: usize = hash as usize % self.hashmap.len();
        while !self.hashmap[idx].is_empty() {
            println!("WARNING: Conflicting hash at index: {}", idx);
            idx += 1;
        }
        self.hashmap[idx] = path;
    }
    pub fn finalize(&mut self) {
        let mut i = 0;
        if self.hashmap[self.hashmap.len() - 1].is_empty() {
            while self.hashmap[i].is_empty() {
                i += 1;
            }
        } else {
            i = self.hashmap.len() - 1;
        }
        let mut target = self.hashmap[i].to_string();
        for i in (0..self.hashmap.len()).rev() {
            if self.hashmap[i].is_empty() {
                self.hashmap[i] = target.to_string();
            } else {
                target = self.hashmap[i].to_string();
            }
        }
        self.finalized = true;
    }
    pub fn clear(&mut self) {
        self.hashmap = vec!["".to_string(); self.size];
        self.finalized = false;
    }
    pub fn find_entity(&mut self, item: &String) -> String {
        self.count += 1;
        let hash: usize = hash_32(item) as usize;
        let mut idx = hash % self.hashmap.len();
        //println!("Finding at initial index {}...", idx);
        self.tries += 1;
        while self.hashmap[idx].is_empty() {
            idx += 1;
            if idx >= self.hashmap.len() {
                //println!("INFO: Search wrapped.");
                idx = 0;
            }
            self.tries += 1;
        }
        let cidx = self.hashmap[idx].chars().position(|c| c == '-').unwrap();
        self.hashmap[idx].substring(0, cidx).to_string()
    }

    pub fn remove(&mut self, item: &String) {
        println!("Removing {}.", item);
        for n in 0..self.hashmap.len() {
            if self.hashmap[n].starts_with(item) {
                self.hashmap[n] = "".to_string();
            }
        }
    }

    pub fn print_stats(&self) {
        println!(
            "{} requests with {} tries, {:.02} tries/request.",
            self.count,
            self.tries,
            self.tries as f64 / self.count as f64
        );
    }
}

fn hash_32(item: &String) -> u32 {
    let a = Sha256::digest(item);
    let d: [u8; 4] = a[a.len() - 4..a.len()].try_into().unwrap();
    u32::from_be_bytes(d)
}

fn extract_disk(path: &str) -> u32 {
    let cidx = path.chars().count() - path.chars().rev().position(|c| c == '/').unwrap();
    path.substring(cidx, path.len())
        .to_string()
        .parse()
        .unwrap()
}

fn modulo_hash(file: &String, num_disks: u32) -> u32 {
    hash_32(file) % num_disks
}

fn cons_add_disks(ch: &mut ConsistentHash, disks: u32, weight: u32) {
    for d in 0..disks {
        for w in 0..weight {
            let path = format!("/{}-{:03}", d, w);
            ch.add(path);
        }
    }
}

fn main() {
    let mut ch = ConsistentHash::new(DISKS * WEIGHT * VEC_MULT);
    cons_add_disks(&mut ch, DISKS, WEIGHT);
    ch.finalize();
    let mut files = Vec::with_capacity(NUM_FILES as usize);
    let mut c_nor = Vec::with_capacity(NUM_FILES as usize);
    let mut m_nor = Vec::with_capacity(NUM_FILES as usize);
    let mut c_less = Vec::with_capacity(NUM_FILES as usize);
    let mut m_less = Vec::with_capacity(NUM_FILES as usize);
    let mut c_more = Vec::with_capacity(NUM_FILES as usize);
    let mut m_more = Vec::with_capacity(NUM_FILES as usize);
    let mut c_nor_d = vec![0; (DISKS + 1) as usize];
    let mut m_nor_d = vec![0; (DISKS + 1) as usize];
    let mut c_less_d = vec![0; (DISKS + 1) as usize];
    let mut m_less_d = vec![0; (DISKS + 1) as usize];
    let mut c_more_d = vec![0; (DISKS + 1) as usize];
    let mut m_more_d = vec![0; (DISKS + 1) as usize];
    let mut c_less_c = 0;
    let mut m_less_c = 0;
    let mut c_more_c = 0;
    let mut m_more_c = 0;

    // Get the distribution with normal devices online
    for nf in 0..NUM_FILES as usize {
        let s: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();
        files.push(s);
        let target = extract_disk(&ch.find_entity(&files[nf]));
        c_nor.push(target);
        c_nor_d[target as usize] += 1;
        let target = modulo_hash(&files[nf], DISKS);
        m_nor.push(target);
        m_nor_d[target as usize] += 1;
    }

    // Get the distribution if one device goes offline
    let rdisk = rand::thread_rng().gen_range(0..DISKS);
    ch.remove(&format!("/mnt/cache/{}", rdisk));
    ch.finalize();
    for nf in 0..NUM_FILES as usize {
        let target = extract_disk(&ch.find_entity(&files[nf]));
        c_less.push(target);
        c_less_d[target as usize] += 1;
        let target = modulo_hash(&files[nf], DISKS - 1);
        m_less.push(target);
        m_less_d[target as usize] += 1;
        if c_less[nf] != c_nor[nf] {
            c_less_c += 1;
        }
        if m_less[nf] != m_nor[nf] {
            m_less_c += 1;
        }
    }

    // Get the distribution if one device is added to the normal set
    ch.clear();
    cons_add_disks(&mut ch, DISKS + 1, WEIGHT);
    ch.finalize();
    for nf in 0..NUM_FILES as usize {
        let target = extract_disk(&ch.find_entity(&files[nf]));
        c_more.push(target);
        c_more_d[target as usize] += 1;
        let target = modulo_hash(&files[nf], DISKS + 1);
        m_more.push(target);
        m_more_d[target as usize] += 1;
        if c_more[nf] != c_nor[nf] {
            c_more_c += 1;
        }
        if m_more[nf] != m_nor[nf] {
            m_more_c += 1;
        }
    }

    // Print out the staticstics
    println!(
        "Number of files: {} Optimal files per disk: {:.2}",
        NUM_FILES,
        NUM_FILES as f64 / DISKS as f64
    );
    println!("File\t\tC\tM\tCl\tMl\tCm\tMm");
    for nf in 0..NUM_FILES as usize {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            files[nf], c_nor[nf], m_nor[nf], c_less[nf], m_less[nf], c_more[nf], m_more[nf]
        );
    }
    println!(
        "Changes\t\t--\t--\t{} {}%\t{} {}%\t{} {}%\t{} {}%",
        c_less_c,
        c_less_c * 100 / NUM_FILES,
        m_less_c,
        m_less_c * 100 / NUM_FILES,
        c_more_c,
        c_more_c * 100 / NUM_FILES,
        m_more_c,
        m_more_c * 100 / NUM_FILES
    );
    for d in 0..(DISKS) as usize {
        println!(
            "Disk {}\t\t{}\t{}\t{}\t{}\t{}\t{}",
            d, c_nor_d[d], m_nor_d[d], c_less_d[d], m_less_d[d], c_more_d[d], m_more_d[d]
        );
    }
    println!(
        "Disk {}\t\t-\t-\t-\t-\t{}\t{}",
        DISKS as usize, c_more_d[DISKS as usize], m_more_d[DISKS as usize]
    );
    ch.print_stats();
    //println!("Hashmap: {:#?}", ch);
}
