use consistent_hash::{DefaultHash, Node, StaticHashRing};
use murmur3::murmur3_32;
use rand::{distributions::Alphanumeric, Rng};
use std::io::Cursor;

const DISKS: u8 = 12;
const WEIGHT: u8 = 160;
const NUM_FILES: u32 = 100000000;
// Limit the number of files displayed.
const LIMIT_FILES: u32 = 100;

struct FileData {
    file: String,
    consistent: u8,
    modulo: u8,
    lconsistent: u8,
    lmodulo: u8,
    mconsistent: u8,
    mmodulo: u8,
}

#[derive(Clone, Default)]
struct DiskData {
    consistent: u32,
    modulo: u32,
    lconsistent: u32,
    lmodulo: u32,
    mconsistent: u32,
    mmodulo: u32,
}

struct MyChash<'a> {
    ring: StaticHashRing<'a, u8, (), DefaultHash>,
}

impl MyChash<'_> {
    pub fn new(disks: Vec<Node<u8, ()>>) -> MyChash<'static> {
        MyChash {
            ring: StaticHashRing::new(DefaultHash, disks.into_iter()),
        }
    }

    pub fn get_disk(&self, file: &String) -> u8 {
        self.ring
            .calc_candidates(file)
            .next()
            .expect("No more disks!")
            .key
    }
}

fn hash_mur32(item: &String, weight: u32) -> u32 {
    murmur3_32(&mut Cursor::new(item), weight).expect("Could not hash!")
}

fn modulo_hash(file: &String, num_disks: u8) -> u8 {
    (hash_mur32(file, 0) % num_disks as u32).try_into().unwrap()
}

fn add_disks(nodes: &mut Vec<Node<u8, ()>>, disks: u8, weight: u8) {
    add_disks_missing(nodes, disks, weight, None);
}

fn add_disks_missing(nodes: &mut Vec<Node<u8, ()>>, disks: u8, weight: u8, missing: Option<u8>) {
    for d in 0..disks {
        match missing {
            None => (),
            Some(mis) => {
                if mis == d {
                    continue;
                }
            }
        }
        nodes.push(Node::new(d).quantity(weight as usize));
    }
}

fn main() {
    let mut nodes = Vec::<Node<u8, ()>>::new();
    add_disks(&mut nodes, DISKS, WEIGHT);
    let c_ring = MyChash::new(nodes);

    let rdisk: u8 = rand::thread_rng().gen_range(0..DISKS);
    println!("Removing disk: {}", rdisk);
    //    my_ring.fail_disk(rdisk);
    let mut nodes = Vec::new();
    add_disks_missing(&mut nodes, DISKS + 1, WEIGHT, Some(rdisk));
    let l_ring = MyChash::new(nodes);

    let mut nodes = Vec::new();
    add_disks(&mut nodes, DISKS + 1, WEIGHT);
    let m_ring = MyChash::new(nodes);

    let mut files: Vec<FileData> = Vec::with_capacity(NUM_FILES as usize);
    let mut disks = vec![DiskData::default(); DISKS as usize + 1];
    let mut c_less_c = 0;
    let mut m_less_c = 0;
    let mut c_more_c = 0;
    let mut m_more_c = 0;

    // Get the distribution with normal devices online
    for _ in 0..NUM_FILES as usize {
        let s: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();

        let ctarget = c_ring.get_disk(&s);
        let lctarget = l_ring.get_disk(&s);
        let mctarget = m_ring.get_disk(&s);
        disks[ctarget as usize].consistent += 1;
        disks[lctarget as usize].lconsistent += 1;
        disks[mctarget as usize].mconsistent += 1;

        let mtarget = modulo_hash(&s, DISKS);
        let lmtarget = modulo_hash(&s, DISKS - 1);
        let mmtarget = modulo_hash(&s, DISKS + 1);
        disks[mtarget as usize].modulo += 1;
        disks[lmtarget as usize].modulo += 1;
        disks[mmtarget as usize].modulo += 1;

        if lctarget != ctarget {
            c_less_c += 1;
        }
        if lmtarget != mtarget {
            m_less_c += 1;
        }
        if mctarget != ctarget {
            c_more_c += 1;
        }
        if mmtarget != mtarget {
            m_more_c += 1;
        }

        files.push(FileData {
            file: s,
            consistent: ctarget,
            modulo: mtarget,
            lconsistent: lctarget,
            lmodulo: lmtarget,
            mconsistent: mctarget,
            mmodulo: mmtarget,
        });
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
    for file in files.iter().take(num_files as usize) {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            file.file,
            file.consistent,
            file.modulo,
            file.lconsistent,
            file.lmodulo,
            file.mconsistent,
            file.mmodulo
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
    for (disk, info) in disks.iter().enumerate().take((DISKS) as usize) {
        println!(
            "Disk {}\t\t{}\t{}\t{}\t{}\t{}\t{}",
            disk,
            info.consistent,
            info.modulo,
            info.lconsistent,
            info.lmodulo,
            info.mconsistent,
            info.mmodulo
        );
    }
    println!(
        "Disk {}\t\t-\t-\t-\t-\t{}\t{}",
        DISKS as usize, disks[DISKS as usize].mconsistent, disks[DISKS as usize].mmodulo
    );
}
