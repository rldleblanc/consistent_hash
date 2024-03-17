use consistent_hash::{DefaultHash, Node, StaticHashRing};
use murmur3::murmur3_32;
use rand::{distributions::Alphanumeric, Rng};
use std::io::Cursor;

const DISKS: u8 = 12;
const WEIGHT: u8 = 160;
const NUM_FILES: u32 = 100000000;
// Limit the number of files displayed.
const LIMIT_FILES: u32 = 100;

struct MyChash<'a> {
    ring: StaticHashRing<'a, u8, (), DefaultHash>,
    bad_disks: Vec<u8>,
}

impl MyChash<'_> {
    pub fn new(disks: Vec<Node<u8, ()>>) -> MyChash<'static> {
        MyChash {
            ring: StaticHashRing::new(DefaultHash, disks.into_iter()),
            bad_disks: Vec::new(),
        }
    }

    pub fn fail_disk(&mut self, disk: u8) {
        self.bad_disks.push(disk);
        self.bad_disks.sort();
    }

    pub fn get_disk(&self, file: &String) -> Option<u8> {
        for candidate in self.ring.calc_candidates(file) {
            let disk = self.bad_disks.binary_search(&candidate.key);
            match disk {
                Err(_) => return Some(candidate.key),
                _ => {
                    //println!("Skipping failed disk {}!", candidate.key);
                    continue;
                }
            };
        }
        None
    }
}

fn hash_mur32(item: &String, weight: u32) -> u32 {
    murmur3_32(&mut Cursor::new(item), weight).expect("Could not hash!")
}

fn modulo_hash(file: &String, num_disks: u8) -> u8 {
    (hash_mur32(file, 0) % num_disks as u32).try_into().unwrap()
}

fn add_disks(nodes: &mut Vec<Node<u8, ()>>, disks: u8, weight: u8) {
    for d in 0..disks {
        nodes.push(Node::new(d).quantity(weight as usize));
    }
}

fn main() {
    let mut nodes = Vec::new();
    add_disks(&mut nodes, DISKS, WEIGHT);
    let mut my_ring = MyChash::new(nodes);
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
        let target = my_ring.get_disk(&files[nf]).unwrap();
        c_nor.push(target);
        c_nor_d[target as usize] += 1;
        let target = modulo_hash(&files[nf], DISKS);
        m_nor.push(target);
        m_nor_d[target as usize] += 1;
    }

    // Get the distribution if one device goes offline
    let rdisk: u8 = rand::thread_rng().gen_range(0..DISKS);
    println!("Removing disk: {}", rdisk);
    my_ring.fail_disk(rdisk);
    for nf in 0..NUM_FILES as usize {
        let target = my_ring.get_disk(&files[nf]).unwrap();
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
    let mut nodes = Vec::new();
    add_disks(&mut nodes, DISKS + 1, WEIGHT);
    let my_ring = MyChash::new(nodes);
    for nf in 0..NUM_FILES as usize {
        let target = my_ring.get_disk(&files[nf]).unwrap();
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
    let mut num_files = NUM_FILES;
    if NUM_FILES > LIMIT_FILES {
        num_files = LIMIT_FILES;
        println!("INFO: Only showing first {} files...", LIMIT_FILES);
    }
    println!("File\t\tC\tM\tCl\tMl\tCm\tMm");
    for nf in 0..num_files as usize {
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
}
