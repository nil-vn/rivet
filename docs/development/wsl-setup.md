# Thiết lập Linux/WSL

## 1. Supported bootstrap environment

Môi trường development đầu tiên của FurrumX:

```text
OS: Linux x86_64
Development host: WSL2
Rust MSRV: 1.94
Pinned development toolchain: 1.97.1
Default Cargo profile: local
```

Linux native cũng được hỗ trợ. Windows native chưa thuộc test matrix ban đầu.

## 2. Vị trí workspace

Functional development có thể chạy từ `/mnt/c`, nhưng build và I/O benchmark tại đây chịu overhead của filesystem bridge giữa Windows và WSL. Không dùng kết quả trên `/mnt/c` làm HPC evidence.

Khuyến nghị đặt clone dùng cho build/benchmark trong WSL Linux filesystem:

```text
/home/<user>/src/furrumx
```

Dataset, `target/`, spill và Parquet output cũng phải nằm trên Linux filesystem hoặc volume/storage được benchmark tường minh.

## 3. System prerequisites

Trên Ubuntu/WSL, toolchain tối thiểu:

```bash
sudo apt update
sudo apt install --yes build-essential pkg-config
```

Các feature về profiling, Protobuf, Python và native build về sau cần:

```bash
sudo apt install --yes clang cmake protobuf-compiler python3 python3-dev
```

Script kiểm tra không tự cài hoặc thay đổi system:

```bash
./scripts/check-wsl.sh
```

## 4. Rust toolchain

Repository pin toolchain trong `rust-toolchain.toml`. Sau khi cài Rust qua rustup chính thức:

```bash
rustup show active-toolchain
rustc --version
cargo --version
```

Toolchain phải có `rustfmt` và `clippy`. `Cargo.toml` khai báo MSRV 1.94 vì optional Wasmtime 47 profile yêu cầu mức này; development/CI dùng 1.97.1 để tạo môi trường tái lập.

## 5. Bootstrap verification

Chạy từ repository root:

```bash
cargo run --locked -- doctor
cargo fmt --all --check
cargo check --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
./scripts/check-docs.sh
```

Minimal control-plane scaffold không kéo Arrow/DataFusion:

```bash
cargo check --locked --no-default-features
```

Default `local` profile compile coherent DataFusion/Arrow/Parquet BOM. Extended feature checks chạy riêng vì thời gian build và native prerequisites:

```bash
cargo check --locked --features flight-sql
cargo check --locked --features wasm
cargo check --locked --features python
cargo check --locked --features s3
cargo check --locked --features distributed
```

## 6. WSL resource configuration

Node low-profile acceptance cần môi trường bị giới hạn thật, không chỉ quan sát process trên máy nhiều RAM. Có thể dùng `.wslconfig` phía Windows hoặc container/cgroup test profile để giới hạn CPU/RAM.

Baseline cần kiểm:

- 2–4 logical CPUs.
- 4–8 GiB RAM.
- Bounded temporary disk.
- Dataset lớn hơn RAM.
- Workspace/output không nằm trên `/mnt/c` khi đo throughput.

Ghi WSL kernel, CPU, RAM, filesystem và mount path vào benchmark manifest.

## 7. Troubleshooting

### Build quá chậm

- Chuyển repository và `target/` khỏi `/mnt/c`.
- Không chạy `--all-features` trong inner development loop.
- Kiểm tra antivirus/indexer trên Windows-mounted path.

### `protoc` không tìm thấy

Cài `protobuf-compiler`; chỉ bắt buộc khi control/distributed protocol build script được bật.

### Python feature không compile

Kiểm tra `python3`, development headers và interpreter mà PyO3 phát hiện. Python không thuộc default local profile.

### Benchmark không ổn định

- Tắt background load có thể kiểm soát.
- Không benchmark trên `/mnt/c`.
- Giữ cùng WSL resource limits và power/host conditions.
- Báo raw samples, không chỉ best run.
