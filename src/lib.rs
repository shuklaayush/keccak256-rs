const NUM_ROUNDS: usize = 24;
const RATE: usize = 1088;
const RATE_BYTES: usize = RATE / 8;
const CAPACITY: usize = 512;

const STATE_SIZE: usize = RATE + CAPACITY;
const NUM_LANES: usize = STATE_SIZE / 64;

// LSB first
fn u8s_to_u64(bytes: &[u8; 8]) -> u64 {
    bytes
        .iter()
        .enumerate()
        .fold(0u64, |acc, (i, &byte)| acc | ((byte as u64) << (i * 8)))
}

// LSB first
fn u64_to_u8s(num: u64) -> [u8; 8] {
    let mut bytes = [0u8; 8];
    for i in 0..8 {
        bytes[i] = ((num >> (i * 8)) & 0xff) as u8;
    }
    bytes
}

// Rotate left
fn ROL64(a: u64, offset: u64) -> u64 {
    (a << offset) ^ (a >> (64 - offset))
}

fn keccakf(lanes: &mut [u64; NUM_LANES]) {
    let mut R = 1u8;
    for _ in 0..NUM_ROUNDS {
        // θ
        let C = (0..5)
            .map(|x| lanes[x] ^ lanes[x + 5] ^ lanes[x + 10] ^ lanes[x + 15] ^ lanes[x + 20])
            .collect::<Vec<u64>>();
        let D = (0..5)
            .map(|x| C[(x + 4) % 5] ^ ROL64(C[(x + 1) % 5], 1))
            .collect::<Vec<u64>>();

        for x in 0..5 {
            for y in 0..5 {
                lanes[x + 5 * y] ^= D[x];
            }
        }

        // ρ and π
        let mut x = 1;
        let mut y = 0;
        let mut current = lanes[1];
        for t in 0..24 {
            let ynext = (2 * x + 3 * y) % 5;
            x = y;
            y = ynext;

            let r = ((t + 1) * (t + 2) / 2) % 64;
            let temp = lanes[x + 5 * y];
            lanes[x + 5 * y] = ROL64(current, r);
            current = temp;
        }
        
        // χ
        for y in 0..5 {
            let mut t = [0u64; 5];
            for x in 0..5 {
                t[x] = lanes[x + 5 * y];
            }
            for x in 0..5 {
                lanes[x + 5 * y] = t[x] ^ ((!t[(x + 1) % 5]) & t[(x + 2) % 5]);
            }
        }

        // ι
        for j in 0..7 {
            R = (R << 1) ^ ((R >> 7) * 0x71);
            if R & 2 == 2 {
                lanes[0] ^= 1 << ((1 << j) - 1);
            }
        }
    }
}

// TODO: fix padding
pub fn keccak256(input: Vec<u8>) -> [u8; 32] {
    // Padding is "10*1" i.e. at least 1 byte (or 2 bits)
    // num_blocks = 1 + floor(l / r) in bytes
    let num_blocks = input.len() / RATE_BYTES + 1;

    let padded_len = num_blocks * RATE_BYTES;
    let mut input_padded = vec![0u8; padded_len];

    for i in 0..input.len() {
        input_padded[i] = input[i];
    }

    // 10*1 padding
    input_padded[input.len()] |= 1 << 7;
    input_padded[padded_len - 1] |= 1;
    
    let input_padded_lane = input_padded
        .chunks_exact(8)
        .map(|chunk| u8s_to_u64(chunk.try_into().unwrap()))
        .collect::<Vec<u64>>(); 
    
    let mut state_in_lanes = [0u64; NUM_LANES];
    for i in 0..num_blocks {
        for j in 0..NUM_LANES {
            if (i * NUM_LANES + j) < input_padded_lane.len() {
                state_in_lanes[j] ^= input_padded_lane[i * NUM_LANES + j];
            }
        }
        keccakf(&mut state_in_lanes);
    }

    let out_bytes = state_in_lanes
        .iter()
        .flat_map(|&lane| u64_to_u8s(lane))
        .collect::<Vec<u8>>();  

    out_bytes[..32].try_into().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccakf() {
        let mut state = [0u64; NUM_LANES];
        keccakf(&mut state);
        println!("{:?}", state);
    }

    #[test]
    fn test_keccak256() {
        let input = vec![61u8; 2];
        let output = keccak256(input);
        println!("{:?}", output);
    }
}
