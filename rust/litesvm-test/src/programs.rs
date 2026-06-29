//! Program artifact discovery and optional mainnet BPF caching.
//!
//! Tests normally load local SBF artifacts built under `target/deploy` or
//! `programs/target/deploy`. Set the `RISE_SDK_LOCALNET_*_SO` environment
//! variables to override those paths. Set
//! [`PHOENIX_MAINNET_BPF_PROGRAMS_ENV`] to `1` or `true` to fetch and cache the
//! deployed mainnet protocol programs for local LiteSVM runs outside CI.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use litesvm::LiteSVM;
use solana_commitment_config::CommitmentConfig;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;

use crate::fixture::{SdkLocalnetFixture, parse_pubkey};

/// When true, tests should panic instead of skipping if program artifacts are
/// missing.
pub const REQUIRED_PROGRAM_ARTIFACT_ENV: &str = "RISE_SDK_LOCALNET_REQUIRE_PROGRAMS";
/// Override the Phoenix repository root used for artifact discovery.
pub const PHOENIX_REPO_ROOT_ENV: &str = "PHOENIX_REPO_ROOT";
/// Override the Phoenix Eternal SBF artifact path.
pub const ETERNAL_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_ETERNAL_SO";
/// Override the Ember SBF artifact path.
pub const EMBER_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_EMBER_SO";
/// Override the Hawkeye SBF artifact path.
pub const HAWKEYE_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_HAWKEYE_SO";
/// Override the Flight SBF artifact path.
pub const FLIGHT_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_FLIGHT_SO";
/// Enable fetching cached mainnet protocol BPFs for non-CI local runs.
pub const PHOENIX_MAINNET_BPF_PROGRAMS_ENV: &str = "PHOENIX_MAINNET_BPF_PROGRAMS";
/// Override the cache directory for fetched mainnet protocol BPFs.
pub const PHOENIX_MAINNET_BPF_PROGRAM_CACHE_DIR_ENV: &str = "PHOENIX_MAINNET_BPF_PROGRAM_CACHE_DIR";
/// Primary RPC URL used when fetching mainnet protocol BPFs.
pub const PHOENIX_MAINNET_RPC_URL_ENV: &str = "PHOENIX_MAINNET_RPC_URL";
/// Fallback RPC URL used when fetching mainnet protocol BPFs.
pub const PHOENIX_RPC_URL_ENV: &str = "PHOENIX_RPC_URL";
/// Default public Solana RPC URL used for mainnet BPF fetches.
pub const DEFAULT_MAINNET_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    solana_pubkey::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
const MAINNET_BPF_CACHE_FINGERPRINT: &str = "mainnet-bpf-programs.fingerprint";
const MAINNET_BPF_CACHE_LOCK: &str = ".mainnet-bpf-programs.lock";
const MAINNET_BPF_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MAINNET_BPF_CACHE_WAIT_TIMEOUT: Duration = Duration::from_secs(300);
const PROGRAMDATA_METADATA_LEN: usize = 45;

#[derive(Clone, Debug)]
/// Local or cached protocol program artifact paths.
pub struct SdkLocalnetProgramPaths {
    pub phoenix_eternal: PathBuf,
    pub ember: PathBuf,
    pub hawkeye: Option<PathBuf>,
    pub flight: Option<PathBuf>,
}

#[derive(Clone, Debug)]
/// Extra program to load into the LiteSVM context.
pub struct SdkLocalnetProgram {
    pub program_id: Pubkey,
    pub path: PathBuf,
}

impl SdkLocalnetProgram {
    /// Construct an extra program descriptor for a program under test.
    pub fn new(program_id: Pubkey, path: impl Into<PathBuf>) -> Self {
        Self {
            program_id,
            path: path.into(),
        }
    }
}

/// Return whether missing SBF artifacts should be treated as test failures.
pub fn sdk_localnet_vm_required() -> bool {
    matches!(
        std::env::var(REQUIRED_PROGRAM_ARTIFACT_ENV).as_deref(),
        Ok("1") | Ok("true")
    )
}

/// Return whether mainnet protocol BPF loading is enabled for this process.
pub fn mainnet_bpf_programs_enabled() -> bool {
    should_load_mainnet_bpf_programs() && !running_in_ci()
}

/// Find protocol program paths for the LiteSVM fixture.
///
/// This first honors explicit `RISE_SDK_LOCALNET_*_SO` paths, then searches
/// common Phoenix build directories. When mainnet BPF mode is enabled outside
/// CI, placeholder paths are returned because the programs are fetched into the
/// cache at context construction time.
pub fn find_sdk_localnet_program_paths() -> Option<SdkLocalnetProgramPaths> {
    if mainnet_bpf_programs_enabled() {
        return Some(SdkLocalnetProgramPaths {
            phoenix_eternal: PathBuf::new(),
            ember: PathBuf::new(),
            hawkeye: None,
            flight: None,
        });
    }

    let explicit = match (
        std::env::var(ETERNAL_PROGRAM_ENV),
        std::env::var(EMBER_PROGRAM_ENV),
        std::env::var(HAWKEYE_PROGRAM_ENV),
        std::env::var(FLIGHT_PROGRAM_ENV),
    ) {
        (Ok(phoenix_eternal), Ok(ember), hawkeye, flight) => Some(SdkLocalnetProgramPaths {
            phoenix_eternal: PathBuf::from(phoenix_eternal),
            ember: PathBuf::from(ember),
            hawkeye: hawkeye.ok().map(PathBuf::from),
            flight: flight.ok().map(PathBuf::from),
        }),
        _ => None,
    };
    if explicit.as_ref().is_some_and(program_paths_exist) {
        return explicit;
    }

    for root in default_program_roots() {
        for program_paths in [
            SdkLocalnetProgramPaths {
                phoenix_eternal: root.join("programs/target/deploy/phoenix_eternal.so"),
                ember: root.join("programs/target/deploy/phoenix_ember_program.so"),
                hawkeye: optional_program_path(
                    root.join("programs/target/deploy/phoenix_hawkeye.so"),
                ),
                flight: optional_program_path(
                    root.join("programs/target/deploy/phoenix_flight.so"),
                ),
            },
            SdkLocalnetProgramPaths {
                phoenix_eternal: root.join("target/deploy/phoenix_eternal.so"),
                ember: root.join("target/deploy/phoenix_ember_program.so"),
                hawkeye: optional_program_path(root.join("target/deploy/phoenix_hawkeye.so")),
                flight: optional_program_path(root.join("target/deploy/phoenix_flight.so")),
            },
            SdkLocalnetProgramPaths {
                phoenix_eternal: root.join("programs/eternal/target/deploy/phoenix_eternal.so"),
                ember: root.join("programs/ember/target/deploy/phoenix_ember_program.so"),
                hawkeye: optional_program_path(
                    root.join("programs/phoenix-hawkeye/target/deploy/phoenix_hawkeye.so"),
                ),
                flight: optional_program_path(
                    root.join("programs/flight/target/deploy/phoenix_flight.so"),
                ),
            },
        ] {
            if program_paths_exist(&program_paths) {
                return Some(program_paths);
            }
        }
    }

    None
}

fn default_program_roots() -> Vec<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = Vec::new();
    if let Ok(root) = std::env::var(PHOENIX_REPO_ROOT_ENV) {
        roots.push(PathBuf::from(root));
    }
    roots.push(manifest_dir.join("../.."));
    roots.push(std::env::current_dir().unwrap_or_else(|_| manifest_dir.clone()));
    roots.dedup();
    roots
}

fn program_paths_exist(paths: &SdkLocalnetProgramPaths) -> bool {
    paths.phoenix_eternal.exists()
        && paths.ember.exists()
        && match paths.hawkeye.as_ref() {
            Some(hawkeye) => hawkeye.exists(),
            None => true,
        }
        && match paths.flight.as_ref() {
            Some(flight) => flight.exists(),
            None => true,
        }
}

fn optional_program_path(path: PathBuf) -> Option<PathBuf> {
    path.exists().then_some(path)
}

pub(crate) fn should_load_mainnet_bpf_programs() -> bool {
    matches!(
        std::env::var(PHOENIX_MAINNET_BPF_PROGRAMS_ENV).as_deref(),
        Ok("1") | Ok("true")
    )
}

pub(crate) fn running_in_ci() -> bool {
    matches!(std::env::var("CI").as_deref(), Ok("1") | Ok("true"))
}

fn mainnet_rpc_url() -> String {
    std::env::var(PHOENIX_MAINNET_RPC_URL_ENV)
        .or_else(|_| std::env::var(PHOENIX_RPC_URL_ENV))
        .unwrap_or_else(|_| DEFAULT_MAINNET_RPC_URL.to_string())
}

pub(crate) fn load_mainnet_protocol_programs(svm: &mut LiteSVM, fixture: &SdkLocalnetFixture) {
    let specs = mainnet_protocol_programs(fixture);
    let cache_dir = mainnet_bpf_program_cache_dir();
    ensure_mainnet_program_cache(&cache_dir, &specs);

    for spec in specs {
        load_program_id(
            svm,
            spec.program_id,
            &mainnet_cached_program_path(&cache_dir, &spec),
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct MainnetProgramSpec {
    name: &'static str,
    program_id: Pubkey,
    file_name: &'static str,
}

fn mainnet_protocol_programs(fixture: &SdkLocalnetFixture) -> Vec<MainnetProgramSpec> {
    vec![
        MainnetProgramSpec {
            name: "phoenix-eternal",
            program_id: parse_pubkey(&fixture.programs.phoenix_eternal)
                .expect("Phoenix program id should parse"),
            file_name: "phoenix_eternal-mainnet.so",
        },
        MainnetProgramSpec {
            name: "ember",
            program_id: parse_pubkey(&fixture.programs.ember)
                .expect("Ember program id should parse"),
            file_name: "phoenix_ember_program-mainnet.so",
        },
        MainnetProgramSpec {
            name: "hawkeye",
            program_id: phoenix_rise_ix::HAWKEYE_PROGRAM_ID,
            file_name: "phoenix_hawkeye-mainnet.so",
        },
        MainnetProgramSpec {
            name: "flight",
            program_id: phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID,
            file_name: "phoenix_flight-mainnet.so",
        },
    ]
}

/// Return the cache directory used for fetched mainnet protocol BPFs.
pub fn mainnet_bpf_program_cache_dir() -> PathBuf {
    if let Ok(cache_dir) = std::env::var(PHOENIX_MAINNET_BPF_PROGRAM_CACHE_DIR_ENV) {
        return PathBuf::from(cache_dir);
    }
    if let Ok(root) = std::env::var(PHOENIX_REPO_ROOT_ENV) {
        return PathBuf::from(root).join("target/deploy/.cache");
    }
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(target_dir).join("deploy/.cache");
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.join("../..");
    if repo_root.join("rise/rust/Cargo.toml").exists() {
        return repo_root.join("target/deploy/.cache");
    }

    if let Ok(current_dir) = std::env::current_dir() {
        if let Some(cargo_root) = find_cargo_target_root(&current_dir) {
            return cargo_root.join("target/deploy/.cache");
        }
    }
    if let Some(cargo_root) = find_cargo_target_root(&manifest_dir) {
        return cargo_root.join("target/deploy/.cache");
    }

    std::env::current_dir()
        .unwrap_or(manifest_dir)
        .join("target/deploy/.cache")
}

fn find_cargo_target_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    let mut first_cargo_manifest = None;

    loop {
        if current.join("Cargo.lock").exists() || current.join("target").exists() {
            return Some(current);
        }
        if current.join("Cargo.toml").exists() && first_cargo_manifest.is_none() {
            first_cargo_manifest = Some(current.clone());
        }
        if !current.pop() {
            break;
        }
    }

    first_cargo_manifest
}

fn ensure_mainnet_program_cache(cache_dir: &Path, specs: &[MainnetProgramSpec]) {
    fs::create_dir_all(cache_dir).unwrap_or_else(|error| {
        panic!("create mainnet BPF cache {}: {error}", cache_dir.display())
    });
    if mainnet_program_cache_is_fresh(cache_dir, specs) {
        return;
    }

    match try_acquire_mainnet_bpf_cache_lock(cache_dir) {
        Ok(Some(_guard)) => {
            if !mainnet_program_cache_is_fresh(cache_dir, specs) {
                fetch_mainnet_program_cache(cache_dir, specs);
            }
        }
        Ok(None) => wait_for_mainnet_program_cache(cache_dir, specs),
        Err(error) => panic!(
            "acquire mainnet BPF cache lock in {}: {error}",
            cache_dir.display()
        ),
    }
}

fn mainnet_program_cache_is_fresh(cache_dir: &Path, specs: &[MainnetProgramSpec]) -> bool {
    if specs
        .iter()
        .any(|spec| !mainnet_cached_program_path(cache_dir, spec).exists())
    {
        return false;
    }

    let fingerprint_path = cache_dir.join(MAINNET_BPF_CACHE_FINGERPRINT);
    let fingerprint = match fs::read_to_string(&fingerprint_path) {
        Ok(fingerprint) => fingerprint,
        Err(_) => return false,
    };
    if specs.iter().any(|spec| {
        !fingerprint.contains(spec.name) || !fingerprint.contains(&spec.program_id.to_string())
    }) {
        return false;
    }

    let modified = match fs::metadata(&fingerprint_path).and_then(|metadata| metadata.modified()) {
        Ok(modified) => modified,
        Err(_) => return false,
    };
    SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default()
        < MAINNET_BPF_CACHE_TTL
}

fn wait_for_mainnet_program_cache(cache_dir: &Path, specs: &[MainnetProgramSpec]) {
    let started = Instant::now();
    while started.elapsed() < MAINNET_BPF_CACHE_WAIT_TIMEOUT {
        if mainnet_program_cache_is_fresh(cache_dir, specs) {
            return;
        }
        let lock_path = cache_dir.join(MAINNET_BPF_CACHE_LOCK);
        if !lock_path.exists() || mainnet_bpf_cache_lock_is_stale(&lock_path) {
            match try_acquire_mainnet_bpf_cache_lock(cache_dir) {
                Ok(Some(_guard)) => {
                    if !mainnet_program_cache_is_fresh(cache_dir, specs) {
                        fetch_mainnet_program_cache(cache_dir, specs);
                    }
                    return;
                }
                Ok(None) => {}
                Err(error) => panic!(
                    "acquire mainnet BPF cache lock in {}: {error}",
                    cache_dir.display()
                ),
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    panic!(
        "timed out waiting for mainnet BPF cache in {}",
        cache_dir.display()
    );
}

struct MainnetBpfCacheLock {
    path: PathBuf,
}

impl Drop for MainnetBpfCacheLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn try_acquire_mainnet_bpf_cache_lock(
    cache_dir: &Path,
) -> std::io::Result<Option<MainnetBpfCacheLock>> {
    let lock_path = cache_dir.join(MAINNET_BPF_CACHE_LOCK);
    match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&lock_path)
    {
        Ok(mut file) => {
            let _ = writeln!(
                file,
                "pid={}\nstarted={}",
                std::process::id(),
                unix_timestamp()
            );
            Ok(Some(MainnetBpfCacheLock { path: lock_path }))
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if mainnet_bpf_cache_lock_is_stale(&lock_path) {
                let _ = fs::remove_file(&lock_path);
                return try_acquire_mainnet_bpf_cache_lock(cache_dir);
            }
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn mainnet_bpf_cache_lock_is_stale(lock_path: &Path) -> bool {
    fs::metadata(lock_path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age > MAINNET_BPF_CACHE_WAIT_TIMEOUT)
}

fn fetch_mainnet_program_cache(cache_dir: &Path, specs: &[MainnetProgramSpec]) {
    let cache_dir = cache_dir.to_path_buf();
    let specs = specs.to_vec();
    let handle = thread::Builder::new()
        .name("rise-mainnet-bpf-fetch".to_string())
        .spawn(move || fetch_mainnet_program_cache_inner(&cache_dir, &specs))
        .expect("spawn mainnet BPF fetch thread");

    if let Err(panic) = handle.join() {
        std::panic::resume_unwind(panic);
    }
}

fn fetch_mainnet_program_cache_inner(cache_dir: &Path, specs: &[MainnetProgramSpec]) {
    let rpc_url = mainnet_rpc_url();
    let client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());
    let mut fingerprint = format!(
        "fetched_at_unix_secs={}\nrpc_url={rpc_url}\n",
        unix_timestamp()
    );

    for spec in specs {
        let bytes = fetch_mainnet_upgradeable_program_bytes(&client, spec);
        write_atomic(&mainnet_cached_program_path(cache_dir, spec), &bytes);
        fingerprint.push_str(&format!(
            "program={} id={} file={} bytes={}\n",
            spec.name,
            spec.program_id,
            spec.file_name,
            bytes.len()
        ));
    }

    write_atomic(
        &cache_dir.join(MAINNET_BPF_CACHE_FINGERPRINT),
        fingerprint.as_bytes(),
    );
}

fn fetch_mainnet_upgradeable_program_bytes(
    client: &RpcClient,
    spec: &MainnetProgramSpec,
) -> Vec<u8> {
    let programdata_address =
        Pubkey::find_program_address(&[spec.program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID).0;
    let programdata_account = client
        .get_account(&programdata_address)
        .unwrap_or_else(|error| {
            panic!(
                "fetch mainnet programdata {} for {} ({}): {error}",
                programdata_address, spec.name, spec.program_id
            )
        });
    if programdata_account.data.len() <= PROGRAMDATA_METADATA_LEN {
        panic!(
            "mainnet programdata {} for {} is too small",
            programdata_address, spec.name
        );
    }
    programdata_account.data[PROGRAMDATA_METADATA_LEN..].to_vec()
}

fn mainnet_cached_program_path(cache_dir: &Path, spec: &MainnetProgramSpec) -> PathBuf {
    cache_dir.join(spec.file_name)
}

fn write_atomic(path: &Path, data: &[u8]) {
    let file_name = path
        .file_name()
        .unwrap_or_else(|| panic!("cache path should have a file name: {}", path.display()))
        .to_string_lossy();
    let tmp = path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id()));
    fs::write(&tmp, data).unwrap_or_else(|error| panic!("write {}: {error}", tmp.display()));
    fs::rename(&tmp, path)
        .unwrap_or_else(|error| panic!("rename {} to {}: {error}", tmp.display(), path.display()));
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn load_program(svm: &mut LiteSVM, program_id: &str, path: &Path) {
    load_program_id(
        svm,
        parse_pubkey(program_id).expect("program id should parse"),
        path,
    );
}

fn load_program_id(svm: &mut LiteSVM, program_id: Pubkey, path: &Path) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    svm.add_program(program_id, &bytes)
        .unwrap_or_else(|error| panic!("failed to load {}: {error:?}", path.display()));
}
