use std::io::{Read, Write};
use std::net::TcpStream;

const TLS_VERSION: u16 = 0x0303;
const TLS_RECORD_HANDSHAKE: u8 = 22;
const TLS_RECORD_APP_DATA: u8 = 23;
const TLS_RECORD_CCS: u8 = 20;
const TLS_HANDSHAKE_CLIENT_HELLO: u8 = 1;
const TLS_HANDSHAKE_SERVER_HELLO: u8 = 2;
const TLS_HANDSHAKE_CERTIFICATE: u8 = 11;
const TLS_HANDSHAKE_SERVER_HELLO_DONE: u8 = 14;
const TLS_HANDSHAKE_CLIENT_KEY_EXCHANGE: u8 = 16;
const TLS_HANDSHAKE_FINISHED: u8 = 20;
const TLS_CIPHER_RSA_AES128_GCM_SHA256: [u8; 2] = [0x00, 0x9C];

pub struct TlsStream {
    stream: TcpStream,
    client_write_key: [u8; 16],
    server_write_key: [u8; 16],
    client_write_iv: [u8; 4],
    server_write_iv: [u8; 4],
    client_seq: u64,
    server_seq: u64,
    read_buf: Vec<u8>,
}

impl TlsStream {
    pub fn connect(host: &str, port: u16) -> Result<Self, String> {
        let stream = TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| format!("TCP connect to {}:{} failed: {}", host, port, e))?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(60)))
            .map_err(|e| format!("set read timeout: {}", e))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(60)))
            .map_err(|e| format!("set write timeout: {}", e))?;

        let mut client_random = [0u8; 32];
        fill_random(&mut client_random);

        let mut tls = TlsStream {
            stream,
            client_write_key: [0u8; 16],
            server_write_key: [0u8; 16],
            client_write_iv: [0u8; 4],
            server_write_iv: [0u8; 4],
            client_seq: 0,
            server_seq: 0,
            read_buf: Vec::new(),
        };

        let mut sent_msgs = Vec::new();
        tls.send_client_hello(host, &client_random, &mut sent_msgs)?;
        let (server_random, cert_bytes, mut recv_msgs) = tls.read_server_hello_cert_done()?;

        let (n, e) = parse_rsa_pubkey_from_cert(&cert_bytes)?;

        let mut pre_master = [0u8; 48];
        pre_master[0] = 0x03;
        pre_master[1] = 0x03;
        fill_random(&mut pre_master[2..]);

        let enc_pms = rsa_encrypt(&pre_master, &n, &e)?;

        let master_secret =
            tls_prf(&pre_master, b"master secret", &[&client_random, &server_random], 48);

        let key_block =
            tls_prf(&master_secret, b"key expansion", &[&server_random, &client_random], 40);

        tls.client_write_key.copy_from_slice(&key_block[0..16]);
        tls.server_write_key.copy_from_slice(&key_block[16..32]);
        tls.client_write_iv.copy_from_slice(&key_block[32..36]);
        tls.server_write_iv.copy_from_slice(&key_block[36..40]);

        tls.send_client_key_exchange(&enc_pms, &mut sent_msgs)?;
        tls.send_change_cipher_spec()?;

        let mut all_msgs = sent_msgs.clone();
        all_msgs.extend_from_slice(&recv_msgs);
        let verify_data = tls_verify_data(&master_secret, &all_msgs, false);
        tls.send_encrypted_handshake(TLS_HANDSHAKE_FINISHED, &verify_data)?;

        tls.read_change_cipher_spec_plain()?;
        recv_msgs.extend_from_slice(&tls.read_encrypted_handshake()?);

        let server_vd = tls_verify_data(&master_secret, &all_msgs, true);
        let fin_idx = recv_msgs.len() - 12;
        if fin_idx < recv_msgs.len() && recv_msgs[fin_idx..] != server_vd {
            return Err("Server Finished verify_data mismatch".into());
        }

        Ok(tls)
    }

    fn send_client_hello(
        &mut self, host: &str, client_random: &[u8; 32], sent: &mut Vec<u8>,
    ) -> Result<(), String> {
        let mut body = Vec::new();
        body.push(TLS_HANDSHAKE_CLIENT_HELLO);
        let mut inner = Vec::new();
        inner.extend_from_slice(&(TLS_VERSION).to_be_bytes());
        inner.extend_from_slice(client_random);
        inner.push(32);
        inner.extend_from_slice(&[0u8; 32]);
        inner.extend_from_slice(&[0x00, 0x02]);
        inner.extend_from_slice(&TLS_CIPHER_RSA_AES128_GCM_SHA256);
        inner.push(1);
        inner.push(0);
        inner.extend_from_slice(&build_sni_extension(host));
        body.extend_from_slice(&u24(inner.len() as u32));
        body.extend_from_slice(&inner);

        sent.extend_from_slice(&body);
        self.send_plaintext_record(TLS_RECORD_HANDSHAKE, &body)
    }

    fn read_server_hello_cert_done(&mut self) -> Result<([u8; 32], Vec<u8>, Vec<u8>), String> {
        let mut server_random = [0u8; 32];
        let mut cert_bytes = Vec::new();
        let mut all_msgs = Vec::new();
        let mut phases = 0u8;

        loop {
            let (ct, body) = self.read_plaintext_record()?;
            if ct != TLS_RECORD_HANDSHAKE {
                return Err(format!("Expected handshake record, got type {}", ct));
            }
            let mut pos = 0usize;
            while pos + 4 <= body.len() {
                let msg_type = body[pos];
                let msg_len = u24_read(&body[pos + 1..pos + 4]) as usize;
                if pos + 4 + msg_len > body.len() {
                    break;
                }
                let msg = &body[pos + 4..pos + 4 + msg_len];
                all_msgs.extend_from_slice(&body[pos..pos + 4 + msg_len]);
                pos += 4 + msg_len;

                match msg_type {
                    TLS_HANDSHAKE_SERVER_HELLO => {
                        if msg.len() < 38 { return Err("ServerHello too short".into()); }
                        server_random.copy_from_slice(&msg[6..38]);
                        phases |= 1;
                    }
                    TLS_HANDSHAKE_CERTIFICATE => {
                        if msg.len() < 7 { return Err("Certificate too short".into()); }
                        let certs_len = u24_read(&msg[0..3]) as usize;
                        let certs_data = &msg[3..];
                        let mut cp = 0usize;
                        while cp + 3 <= certs_data.len() && cp < certs_len {
                            let clen = u24_read(&certs_data[cp..cp + 3]) as usize;
                            cp += 3;
                            if cp + clen > certs_data.len() { break; }
                            if cert_bytes.is_empty() {
                                cert_bytes = certs_data[cp..cp + clen].to_vec();
                            }
                            cp += clen;
                        }
                        phases |= 2;
                    }
                    TLS_HANDSHAKE_SERVER_HELLO_DONE => {
                        phases |= 4;
                    }
                    _ => {}
                }
            }

            if phases & 4 != 0 {
                break;
            }
        }

        if cert_bytes.is_empty() {
            return Err("No certificate in ServerHello".into());
        }
        Ok((server_random, cert_bytes, all_msgs))
    }

    fn send_client_key_exchange(
        &mut self, enc_pms: &[u8], sent: &mut Vec<u8>,
    ) -> Result<(), String> {
        let mut body = Vec::new();
        body.push(TLS_HANDSHAKE_CLIENT_KEY_EXCHANGE);
        body.extend_from_slice(&u24(enc_pms.len() as u32 + 2));
        body.extend_from_slice(&((enc_pms.len() as u16).to_be_bytes()));
        body.extend_from_slice(enc_pms);
        sent.extend_from_slice(&body);
        self.send_plaintext_record(TLS_RECORD_HANDSHAKE, &body)
    }

    fn send_change_cipher_spec(&mut self) -> Result<(), String> {
        self.stream.write_all(&[TLS_RECORD_CCS, 0x03, 0x03, 0x00, 0x01, 0x01])
            .map_err(|e| format!("send CCS: {}", e))?;
        Ok(())
    }

    fn send_encrypted_handshake(
        &mut self, msg_type: u8, data: &[u8],
    ) -> Result<(), String> {
        let mut body = Vec::new();
        body.push(msg_type);
        body.extend_from_slice(&u24(data.len() as u32));
        body.extend_from_slice(data);
        self.send_encrypted_record(TLS_RECORD_HANDSHAKE, &body)
    }

    fn read_change_cipher_spec_plain(&mut self) -> Result<(), String> {
        let mut buf = [0u8; 6];
        self.stream.read_exact(&mut buf).map_err(|e| format!("read CCS: {}", e))?;
        Ok(())
    }

    fn read_encrypted_handshake(&mut self) -> Result<Vec<u8>, String> {
        let (ct, body) = self.read_encrypted_record()?;
        if ct != TLS_RECORD_HANDSHAKE {
            return Err(format!("Expected encrypted handshake, got {}", ct));
        }
        if body.len() < 4 { return Err("Handshake too short".into()); }
        let _msg_type = body[0];
        let msg_len = u24_read(&body[1..4]) as usize;
        if body.len() < 4 + msg_len { return Err("Handshake truncated".into()); }
        Ok(body[4..4 + msg_len].to_vec())
    }

    pub fn write_all(&mut self, data: &[u8]) -> Result<(), String> {
        self.send_encrypted_record(TLS_RECORD_APP_DATA, data)
    }

    fn send_plaintext_record(&mut self, ct: u8, data: &[u8]) -> Result<(), String> {
        let header: [u8; 5] = [
            ct,
            (TLS_VERSION >> 8) as u8,
            TLS_VERSION as u8,
            (data.len() >> 8) as u8,
            data.len() as u8,
        ];
        self.stream.write_all(&header)
            .map_err(|e| format!("write header: {}", e))?;
        self.stream.write_all(data)
            .map_err(|e| format!("write body: {}", e))?;
        self.stream.flush()
            .map_err(|e| format!("flush: {}", e))?;
        Ok(())
    }

    fn read_plaintext_record(&mut self) -> Result<(u8, Vec<u8>), String> {
        let mut header = [0u8; 5];
        self.stream.read_exact(&mut header)
            .map_err(|e| format!("read plain header: {}", e))?;
        let ct = header[0];
        let len = ((header[3] as usize) << 8) | (header[4] as usize);
        let mut body = vec![0u8; len];
        self.stream.read_exact(&mut body)
            .map_err(|e| format!("read plain body len={}: {}", len, e))?;
        Ok((ct, body))
    }

    fn send_encrypted_record(&mut self, ct: u8, data: &[u8]) -> Result<(), String> {
        let nonce = build_nonce(&self.client_write_iv, self.client_seq);
        self.client_seq = self.client_seq.wrapping_add(1);

        let aad: [u8; 5] = [
            ct, (TLS_VERSION >> 8) as u8, TLS_VERSION as u8,
            (data.len() >> 8) as u8, data.len() as u8,
        ];
        let (ciphertext, tag) =
            aes128_gcm_encrypt(&self.client_write_key, &nonce, data, &aad);

        let record_len = ciphertext.len() + 16;
        let header: [u8; 5] = [
            ct, (TLS_VERSION >> 8) as u8, TLS_VERSION as u8,
            (record_len >> 8) as u8, record_len as u8,
        ];
        self.stream.write_all(&header)
            .map_err(|e| format!("enc write header: {}", e))?;
        self.stream.write_all(&ciphertext)
            .map_err(|e| format!("enc write body: {}", e))?;
        self.stream.write_all(&tag)
            .map_err(|e| format!("enc write tag: {}", e))?;
        self.stream.flush()
            .map_err(|e| format!("enc flush: {}", e))?;
        Ok(())
    }

    fn read_encrypted_record(&mut self) -> Result<(u8, Vec<u8>), String> {
        let mut header = [0u8; 5];
        self.stream.read_exact(&mut header)
            .map_err(|e| format!("read enc header: {}", e))?;
        let ct = header[0];
        if ct == 21 {
            let mut alert = [0u8; 2];
            self.stream.read_exact(&mut alert).ok();
            return Err(format!("TLS alert: level={} desc={}", alert[0], alert[1]));
        }
        let rec_len = ((header[3] as usize) << 8) | (header[4] as usize);
        let mut body = vec![0u8; rec_len];
        self.stream.read_exact(&mut body)
            .map_err(|e| format!("read enc body len={}: {}", rec_len, e))?;

        let ct_len = rec_len.saturating_sub(16);
        if ct_len == 0 { return Err("TLS record too short for GCM".into()); }
        let tag: [u8; 16] = body[ct_len..].try_into().map_err(|_| "bad tag slice")?;
        let ciphertext = &body[..ct_len];

        let aad: [u8; 5] = [
            ct, (TLS_VERSION >> 8) as u8, TLS_VERSION as u8,
            (ct_len >> 8) as u8, ct_len as u8,
        ];
        let nonce = build_nonce(&self.server_write_iv, self.server_seq);
        self.server_seq = self.server_seq.wrapping_add(1);

        let plain =
            aes128_gcm_decrypt(&self.server_write_key, &nonce, ciphertext, &aad, &tag)?;
        Ok((ct, plain))
    }

    fn fill_read_buf(&mut self) -> Result<(), String> {
        let (ct, data) = self.read_encrypted_record()?;
        if ct == TLS_RECORD_APP_DATA {
            self.read_buf.extend_from_slice(&data);
            Ok(())
        } else {
            Err(format!("Unexpected record type {} in app data", ct))
        }
    }
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        while self.read_buf.is_empty() {
            self.fill_read_buf().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e)
            })?;
        }
        let n = buf.len().min(self.read_buf.len());
        buf[..n].copy_from_slice(&self.read_buf[..n]);
        self.read_buf.drain(..n);
        Ok(n)
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_all(buf).map(|_| buf.len())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

fn build_sni_extension(host: &str) -> Vec<u8> {
    let host_bytes = host.as_bytes();
    let mut ext = Vec::new();
    ext.extend_from_slice(&[0x00, 0x00]);
    let mut sni = Vec::new();
    sni.extend_from_slice(&((host_bytes.len() + 3) as u16).to_be_bytes());
    sni.push(0);
    sni.extend_from_slice(&((host_bytes.len()) as u16).to_be_bytes());
    sni.extend_from_slice(host_bytes);
    ext.extend_from_slice(&((sni.len()) as u16).to_be_bytes());
    ext.extend_from_slice(&sni);
    ext
}

fn build_nonce(iv: &[u8; 4], seq: u64) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[0..4].copy_from_slice(iv);
    nonce[4..12].copy_from_slice(&seq.to_be_bytes());
    nonce
}

fn u24(val: u32) -> [u8; 3] {
    [(val >> 16) as u8, (val >> 8) as u8, val as u8]
}

fn u24_read(data: &[u8]) -> u32 {
    ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32)
}

fn fill_random(buf: &mut [u8]) {
    for (i, b) in buf.iter_mut().enumerate() {
        let micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        *b =
            ((micros >> ((i % 8) * 8)) ^ (micros >> 16) ^ (i as u128 * 0x9E3779B97F4A7C15)) as u8;
    }
}

fn tls_prf(secret: &[u8], label: &[u8], seeds: &[&[u8]], len: usize) -> Vec<u8> {
    let mut seed = Vec::new();
    seed.extend_from_slice(label);
    for s in seeds {
        seed.extend_from_slice(s);
    }
    p_hash(secret, &seed, len)
}

fn p_hash(secret: &[u8], seed: &[u8], len: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(len);
    let mut a = hmac_sha256(secret, seed);
    while result.len() < len {
        let mut inp = a.clone();
        inp.extend_from_slice(seed);
        let h = hmac_sha256(secret, &inp);
        result.extend_from_slice(&h);
        a = hmac_sha256(secret, &a);
    }
    result.truncate(len);
    result
}

fn tls_verify_data(
    master_secret: &[u8],
    handshake_messages: &[u8],
    is_server: bool,
) -> [u8; 12] {
    let label = if is_server {
        b"server finished"
    } else {
        b"client finished"
    };
    let hash = sha256(handshake_messages);
    let vd = tls_prf(master_secret, label, &[&hash], 12);
    let mut out = [0u8; 12];
    out.copy_from_slice(&vd);
    out
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let padded = pad_sha256(data);
    for chunk in padded.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

fn pad_sha256(data: &[u8]) -> Vec<u8> {
    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    padded
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let block_size = 64;
    let mut ikey = vec![0x36u8; block_size];
    let mut okey = vec![0x5Cu8; block_size];
    let k = if key.len() > block_size {
        sha256(key).to_vec()
    } else {
        key.to_vec()
    };
    for (i, &b) in k.iter().enumerate() {
        ikey[i] ^= b;
        okey[i] ^= b;
    }
    let mut inner = ikey;
    inner.extend_from_slice(data);
    let ih = sha256(&inner);
    let mut outer = okey;
    outer.extend_from_slice(&ih);
    sha256(&outer).to_vec()
}

fn rsa_encrypt(data: &[u8; 48], n: &[u8], e: &[u8]) -> Result<Vec<u8>, String> {
    let padded = pkcs1_v15_pad(data, n.len())?;
    let m = bytes_to_biguint(&padded);
    let exp = bytes_to_biguint(e);
    let modulus = bytes_to_biguint(n);
    let c = modpow(&m, &exp, &modulus)?;
    Ok(biguint_to_bytes(&c, n.len()))
}

fn pkcs1_v15_pad(data: &[u8; 48], key_bytes: usize) -> Result<Vec<u8>, String> {
    let ps_len = key_bytes - 3 - data.len();
    let mut padded = vec![0x00u8, 0x02];
    for _ in 0..ps_len {
        padded.push(rand_byte());
    }
    padded.push(0x00);
    padded.extend_from_slice(data);
    Ok(padded)
}

fn rand_byte() -> u8 {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    ((n ^ (n >> 17) ^ (n >> 31)) & 0xFF) as u8 | 1
}

fn bytes_to_biguint(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}

fn biguint_to_bytes(val: &[u8], min_len: usize) -> Vec<u8> {
    let mut out = val.to_vec();
    while out.len() < min_len {
        out.insert(0, 0);
    }
    if out.len() > min_len {
        out.drain(..out.len() - min_len);
    }
    out
}

fn modpow(base: &[u8], exp: &[u8], modulus: &[u8]) -> Result<Vec<u8>, String> {
    let modu = modulus.to_vec();
    let mut result = vec![1u8];
    let mut base_val = base.to_vec();
    for &byte in exp.iter().rev() {
        for bit_pos in 0..8 {
            if (byte >> bit_pos) & 1 == 1 {
                result = big_mul_mod(&result, &base_val, &modu)?;
            }
            base_val = big_mul_mod(&base_val, &base_val, &modu)?;
        }
    }
    Ok(result)
}

fn big_mul_mod(a: &[u8], b: &[u8], m: &[u8]) -> Result<Vec<u8>, String> {
    let a_len = a.len();
    let b_len = b.len();
    let rlen = a_len + b_len;
    let mut result = vec![0u8; rlen];
    for i in 0..a_len {
        let mut carry: u16 = 0;
        for j in 0..b_len {
            let idx = i + j;
            let ri = rlen - 1 - idx;
            let prod = (a[a_len - 1 - i] as u16) * (b[b_len - 1 - j] as u16)
                + (result[ri] as u16)
                + carry;
            result[ri] = (prod & 0xFF) as u8;
            carry = prod >> 8;
        }
        let mut pos = a_len - 1 - i + b_len;
        while carry > 0 && pos < rlen {
            let ri = rlen - 1 - pos;
            let sum = (result[ri] as u16) + carry;
            result[ri] = (sum & 0xFF) as u8;
            carry = sum >> 8;
            pos += 1;
        }
    }
    while result.len() > 1 && result[0] == 0 {
        result.remove(0);
    }
    big_mod(&result, m)
}

fn big_mod(a: &[u8], m: &[u8]) -> Result<Vec<u8>, String> {
    if m.iter().all(|&b| b == 0) {
        return Err("Division by zero".into());
    }
    let mut remainder = Vec::new();
    for &byte in a {
        remainder.push(byte);
        while remainder.len() > 1 && remainder[0] == 0 {
            remainder.remove(0);
        }
        while big_cmp(&remainder, m) >= 0 {
            remainder = big_sub(&remainder, m);
        }
    }
    if remainder.is_empty() {
        remainder.push(0);
    }
    Ok(remainder)
}

fn big_cmp(a: &[u8], b: &[u8]) -> i32 {
    let a_trimmed = skip_leading_zeros(a);
    let b_trimmed = skip_leading_zeros(b);
    if a_trimmed.len() != b_trimmed.len() {
        return (a_trimmed.len() as i32) - (b_trimmed.len() as i32);
    }
    for (x, y) in a_trimmed.iter().zip(b_trimmed) {
        if x != y {
            return (*x as i32) - (*y as i32);
        }
    }
    0
}

fn big_sub(a: &[u8], b: &[u8]) -> Vec<u8> {
    let a_trimmed = skip_leading_zeros(a);
    let b_trimmed = skip_leading_zeros(b);
    let max_len = a_trimmed.len().max(b_trimmed.len());
    let mut result = vec![0u8; max_len];
    let mut borrow: i16 = 0;
    for i in 0..max_len {
        let ai = if i < a_trimmed.len() {
            a_trimmed[a_trimmed.len() - 1 - i] as i16
        } else {
            0
        };
        let bi = if i < b_trimmed.len() {
            b_trimmed[b_trimmed.len() - 1 - i] as i16
        } else {
            0
        };
        let mut diff = ai - bi - borrow;
        if diff < 0 {
            diff += 256;
            borrow = 1;
        } else {
            borrow = 0;
        }
        result[max_len - 1 - i] = diff as u8;
    }
    skip_leading_zeros(&result).to_vec()
}

fn skip_leading_zeros(bytes: &[u8]) -> &[u8] {
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    if start >= bytes.len() {
        &[0]
    } else {
        &bytes[start..]
    }
}

fn parse_rsa_pubkey_from_cert(der: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let cert = parse_der_seq(der, 0)?.1;
    let (_, tbs) = parse_der_seq(cert, 0).map_err(|e| format!("tbsCertificate: {}", e))?;
    let mut pos = 0usize;
    for _ in 0..6 {
        let (next_pos, _) =
            parse_der_any(tbs, pos).map_err(|e| format!("skip field at {}: {}", pos, e))?;
        pos = next_pos;
    }
    let (next_pos, spki) = parse_der_seq(tbs, pos)
        .map_err(|e| format!("subjectPublicKeyInfo at {}: {}", pos, e))?;
    let _pos = next_pos;
    let (next_pos, _) = parse_der_seq(spki, 0)
        .map_err(|e| format!("spki algorithm: {}", e))?;
    let (_, pubkey_bits) = parse_der_bitstring(spki, next_pos)
        .map_err(|e| format!("spki pubkey at {}: {}", next_pos, e))?;
    let (_, rsa_pubkey) = parse_der_seq(&pubkey_bits, 0)
        .map_err(|e| format!("RSAPublicKey: {}", e))?;
    let (_, n) =
        parse_der_integer(rsa_pubkey, 0).map_err(|e| format!("RSA modulus: {}", e))?;
    let (_, e) =
        parse_der_integer(rsa_pubkey, n.len() + 4).map_err(|e| format!("RSA exponent: {}", e))?;
    Ok((n, e))
}

fn parse_der_any(data: &[u8], pos: usize) -> Result<(usize, &[u8]), String> {
    if pos + 2 > data.len() {
        return Err("DER: unexpected EOF".into());
    }
    let len_byte = data[pos + 1];
    let (len, header_len) = if len_byte < 0x80 {
        (len_byte as usize, 2)
    } else {
        let num_bytes = (len_byte & 0x7F) as usize;
        if pos + 2 + num_bytes > data.len() {
            return Err("DER: long length overflow".into());
        }
        let mut l = 0usize;
        for i in 0..num_bytes {
            l = (l << 8) | (data[pos + 2 + i] as usize);
        }
        (l, 2 + num_bytes)
    };
    if pos + header_len + len > data.len() {
        return Err(format!(
            "DER value overflow: pos={} len={} avail={}",
            pos,
            len,
            data.len() - pos
        ));
    }
    let val = &data[pos + header_len..pos + header_len + len];
    Ok((pos + header_len + len, val))
}

fn parse_der_seq(data: &[u8], pos: usize) -> Result<(usize, &[u8]), String> {
    if pos >= data.len() || data[pos] != 0x30 {
        return Err(format!(
            "Expected SEQUENCE (0x30), got 0x{:02X} at {}",
            data.get(pos).unwrap_or(&0),
            pos
        ));
    }
    parse_der_any(data, pos)
}

fn parse_der_integer(data: &[u8], pos: usize) -> Result<(usize, Vec<u8>), String> {
    if pos >= data.len() || data[pos] != 0x02 {
        return Err(format!("Expected INTEGER (0x02) at {}", pos));
    }
    let (next_pos, val) = parse_der_any(data, pos)?;
    let stripped = if val.len() > 1 && val[0] == 0 {
        val[1..].to_vec()
    } else {
        val.to_vec()
    };
    Ok((next_pos, stripped))
}

fn parse_der_bitstring(data: &[u8], pos: usize) -> Result<(usize, Vec<u8>), String> {
    if pos >= data.len() || data[pos] != 0x03 {
        return Err(format!("Expected BIT STRING (0x03) at {}", pos));
    }
    let (next_pos, val) = parse_der_any(data, pos)?;
    if val.is_empty() {
        return Err("Empty BIT STRING".into());
    }
    Ok((next_pos, val[1..].to_vec()))
}

fn aes128_gcm_encrypt(
    key: &[u8; 16],
    nonce: &[u8; 12],
    plaintext: &[u8],
    aad: &[u8],
) -> (Vec<u8>, [u8; 16]) {
    let mut counter = [0u8; 16];
    counter[0..12].copy_from_slice(nonce);
    counter[15] = 2;

    let mut ciphertext = Vec::with_capacity(plaintext.len());
    for chunk in plaintext.chunks(16) {
        let keystream = aes_encrypt_block(key, &counter);
        for (i, &b) in chunk.iter().enumerate() {
            ciphertext.push(b ^ keystream[i]);
        }
        inc_ctr(&mut counter);
    }

    let tag = compute_gcm_tag(key, nonce, &ciphertext, aad);
    (ciphertext, tag)
}

fn aes128_gcm_decrypt(
    key: &[u8; 16],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    aad: &[u8],
    tag: &[u8; 16],
) -> Result<Vec<u8>, String> {
    let mut counter = [0u8; 16];
    counter[0..12].copy_from_slice(nonce);
    counter[15] = 2;

    let mut plaintext = Vec::with_capacity(ciphertext.len());
    for chunk in ciphertext.chunks(16) {
        let keystream = aes_encrypt_block(key, &counter);
        for (i, &b) in chunk.iter().enumerate() {
            plaintext.push(b ^ keystream[i]);
        }
        inc_ctr(&mut counter);
    }

    let computed = compute_gcm_tag(key, nonce, ciphertext, aad);
    if computed != *tag {
        return Err("TLS: GCM authentication failed".into());
    }
    Ok(plaintext)
}

fn compute_gcm_tag(key: &[u8; 16], nonce: &[u8; 12], ciphertext: &[u8], aad: &[u8]) -> [u8; 16] {
    let h = aes_encrypt_block(key, &[0u8; 16]);
    let mut tag_input = Vec::new();
    tag_input.extend_from_slice(aad);
    while tag_input.len() % 16 != 0 {
        tag_input.push(0);
    }
    tag_input.extend_from_slice(ciphertext);
    while tag_input.len() % 16 != 0 {
        tag_input.push(0);
    }
    let mut len_block = [0u8; 16];
    len_block[0..8].copy_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
    len_block[8..16].copy_from_slice(&((ciphertext.len() as u64) * 8).to_be_bytes());
    tag_input.extend_from_slice(&len_block);

    let mut y = [0u8; 16];
    for chunk in tag_input.chunks(16) {
        for (i, b) in y.iter_mut().enumerate() {
            *b ^= chunk[i];
        }
        y = ghash_mul(&y, &h);
    }

    let mut j0 = [0u8; 16];
    j0[0..12].copy_from_slice(nonce);
    j0[15] = 1;
    let enc_j0 = aes_encrypt_block(key, &j0);

    let mut tag = [0u8; 16];
    for i in 0..16 {
        tag[i] = y[i] ^ enc_j0[i];
    }
    tag
}

fn inc_ctr(ctr: &mut [u8; 16]) {
    for i in (0..16).rev() {
        ctr[i] = ctr[i].wrapping_add(1);
        if ctr[i] != 0 {
            break;
        }
    }
}

fn ghash_mul(x: &[u8; 16], h: &[u8; 16]) -> [u8; 16] {
    let mut z = [0u8; 16];
    let mut v = *h;
    for byte_idx in 0..16 {
        for bit in 0..8 {
            if (x[byte_idx] >> (7 - bit)) & 1 == 1 {
                for i in 0..16 {
                    z[i] ^= v[i];
                }
            }
            let lsb = v[15] & 1;
            shift_right(&mut v);
            if lsb == 1 {
                v[0] ^= 0xE1;
            }
        }
    }
    z
}

fn shift_right(v: &mut [u8; 16]) {
    for i in (1..16).rev() {
        v[i] = (v[i] >> 1) | (v[i - 1] << 7);
    }
    v[0] >>= 1;
}

fn aes_encrypt_block(key: &[u8; 16], block: &[u8; 16]) -> [u8; 16] {
    let sbox: [u8; 256] = [
        0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7,
        0xab, 0x76, 0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf,
        0x9c, 0xa4, 0x72, 0xc0, 0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5,
        0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15, 0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a,
        0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75, 0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e,
        0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84, 0x53, 0xd1, 0x00, 0xed,
        0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf, 0xd0, 0xef,
        0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
        0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff,
        0xf3, 0xd2, 0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d,
        0x64, 0x5d, 0x19, 0x73, 0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee,
        0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb, 0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c,
        0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79, 0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5,
        0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08, 0xba, 0x78, 0x25, 0x2e,
        0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a, 0x70, 0x3e,
        0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
        0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55,
        0x28, 0xdf, 0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f,
        0xb0, 0x54, 0xbb, 0x16,
    ];
    let rcon: [u8; 11] = [
        0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
    ];
    let round_keys = expand_key(key, &sbox, &rcon);
    let mut state = *block;
    add_round_key(&mut state, &round_keys[0..16]);
    for round in 1..10 {
        sub_bytes(&mut state, &sbox);
        shift_rows(&mut state);
        mix_columns(&mut state);
        add_round_key(&mut state, &round_keys[round * 16..(round + 1) * 16]);
    }
    sub_bytes(&mut state, &sbox);
    shift_rows(&mut state);
    add_round_key(&mut state, &round_keys[160..176]);
    state
}

fn expand_key(key: &[u8; 16], sbox: &[u8; 256], rcon: &[u8; 11]) -> [u8; 176] {
    let mut w = [0u8; 176];
    w[0..16].copy_from_slice(key);
    for i in 4..44 {
        let mut t = [0u8; 4];
        t.copy_from_slice(&w[(i - 1) * 4..i * 4]);
        if i % 4 == 0 {
            t.rotate_left(1);
            for b in t.iter_mut() {
                *b = sbox[*b as usize];
            }
            t[0] ^= rcon[i / 4];
        }
        for j in 0..4 {
            w[i * 4 + j] = w[(i - 4) * 4 + j] ^ t[j];
        }
    }
    w
}

fn add_round_key(state: &mut [u8; 16], key: &[u8]) {
    for i in 0..16 {
        state[i] ^= key[i];
    }
}

fn sub_bytes(state: &mut [u8; 16], sbox: &[u8; 256]) {
    for b in state.iter_mut() {
        *b = sbox[*b as usize];
    }
}

fn shift_rows(state: &mut [u8; 16]) {
    let s = *state;
    for i in 0..4 {
        for j in 0..4 {
            state[j * 4 + i] = s[j * 4 + ((i + j) % 4)];
        }
    }
}

fn mix_columns(state: &mut [u8; 16]) {
    for col in 0..4 {
        let base = col * 4;
        let a = state[base..base + 4].to_vec();
        state[base] = gmul(2, a[0]) ^ gmul(3, a[1]) ^ a[2] ^ a[3];
        state[base + 1] = a[0] ^ gmul(2, a[1]) ^ gmul(3, a[2]) ^ a[3];
        state[base + 2] = a[0] ^ a[1] ^ gmul(2, a[2]) ^ gmul(3, a[3]);
        state[base + 3] = gmul(3, a[0]) ^ a[1] ^ a[2] ^ gmul(2, a[3]);
    }
}

fn gmul(a: u8, b: u8) -> u8 {
    let (mut p, mut a_val, mut b_val) = (0u8, a, b);
    for _ in 0..8 {
        if b_val & 1 != 0 {
            p ^= a_val;
        }
        let hi = a_val & 0x80;
        a_val <<= 1;
        if hi != 0 {
            a_val ^= 0x1b;
        }
        b_val >>= 1;
    }
    p
}
