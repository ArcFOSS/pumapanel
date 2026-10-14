//! Pure builders for the `docker`/`podman` command lines.
//!
//! Both engines accept the same subcommands and flags, so one set of builders
//! serves either runtime. Nothing here executes anything: every function
//! returns a plain [`Invocation`], which keeps the command shape unit-testable
//! and means the agent never builds a shell string (no shell, no injection).

use std::path::{Path, PathBuf};

/// Marks every container the agent owns, so it can list them back.
pub const MANAGED_LABEL: &str = "pumapanel.managed";
/// Carries PumaPanel's server id on the container.
pub const SERVER_LABEL: &str = "pumapanel.server";
/// Records the Minecraft version the container was created with.
pub const VERSION_LABEL: &str = "pumapanel.version";
/// The port Minecraft listens on inside the container.
pub const SERVER_PORT: u16 = 25565;
/// Grace period given to `stop`, so the server can save its world.
const STOP_TIMEOUT_SECS: u32 = 30;

/// A single container-runtime invocation: a program plus its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl Invocation {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, value: impl Into<String>) -> Self {
        self.args.push(value.into());
        self
    }
}

/// Everything needed to create one PaperMC server container.
#[derive(Debug, Clone)]
pub struct CreateSpec<'a> {
    pub id: &'a str,
    pub version: &'a str,
    pub memory_mb: u32,
    pub host_port: u16,
    pub image: &'a str,
}

/// Container name for a server id.
pub fn container_name(id: &str) -> String {
    format!("pumapanel-{id}")
}

/// Named volume holding the server's world data.
pub fn volume_name(id: &str) -> String {
    format!("pumapanel-{id}")
}

/// Whether `id` is safe to embed in container and volume names.
///
/// Ids come from the API (UUIDs), but the agent validates them itself rather
/// than trusting the caller: a name containing `/` or whitespace would be
/// reinterpreted by the runtime CLI.
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Creates and starts the server container.
pub fn create(binary: &Path, spec: &CreateSpec<'_>) -> Invocation {
    Invocation::new(binary)
        .arg("run")
        .arg("--detach")
        .arg("--name")
        .arg(container_name(spec.id))
        .arg("--label")
        .arg(format!("{MANAGED_LABEL}=true"))
        .arg("--label")
        .arg(format!("{SERVER_LABEL}={}", spec.id))
        .arg("--label")
        .arg(format!("{VERSION_LABEL}={}", spec.version))
        .arg("--restart")
        .arg("unless-stopped")
        .arg("--env")
        .arg("EULA=TRUE")
        .arg("--env")
        .arg("TYPE=PAPER")
        .arg("--env")
        .arg(format!("VERSION={}", spec.version))
        // The itzg image sizes the JVM heap from MEMORY/INIT_MEMORY.
        .arg("--env")
        .arg(format!("MEMORY={}M", spec.memory_mb))
        .arg("--env")
        .arg(format!("INIT_MEMORY={}M", spec.memory_mb))
        .arg("--publish")
        .arg(format!("{}:{SERVER_PORT}", spec.host_port))
        .arg("--volume")
        .arg(format!("{}:/data", volume_name(spec.id)))
        .arg(spec.image)
}

pub fn start(binary: &Path, id: &str) -> Invocation {
    Invocation::new(binary).arg("start").arg(container_name(id))
}

pub fn stop(binary: &Path, id: &str) -> Invocation {
    Invocation::new(binary)
        .arg("stop")
        .arg("--time")
        .arg(STOP_TIMEOUT_SECS.to_string())
        .arg(container_name(id))
}

/// Force-removes the container, running or not.
pub fn remove(binary: &Path, id: &str) -> Invocation {
    Invocation::new(binary)
        .arg("rm")
        .arg("--force")
        .arg(container_name(id))
}

/// Removes the server's data volume.
pub fn remove_volume(binary: &Path, id: &str) -> Invocation {
    Invocation::new(binary)
        .arg("volume")
        .arg("rm")
        .arg(volume_name(id))
}

/// Reads one container's state by container name. The output is a JSON array.
pub fn inspect(binary: &Path, container: &str) -> Invocation {
    Invocation::new(binary).arg("inspect").arg(container)
}

/// Lists the names of every managed container.
pub fn list_managed(binary: &Path) -> Invocation {
    Invocation::new(binary)
        .arg("ps")
        .arg("--all")
        .arg("--filter")
        .arg(format!("label={MANAGED_LABEL}=true"))
        .arg("--format")
        .arg("{{.Names}}")
}

/// Reports the runtime's version.
pub fn version(binary: &Path) -> Invocation {
    Invocation::new(binary).arg("--version")
}

/// Picks the image tag for a PaperMC version's minimum Java.
///
/// `itzg/minecraft-server` publishes `java8`/`java11`/`java16`/`java17`/
/// `java21`/`java25` tags; PaperMC reports a minimum Java for each of its
/// versions and they line up almost exactly. An image that already names a tag
/// (for example `registry.internal:5000/minecraft-server:stable-java21`) is used
/// untouched, which is the escape hatch for private mirrors.
pub fn image_for_java(image: &str, java_min: Option<u8>) -> String {
    if has_tag(image) {
        return image.to_string();
    }
    match java_min.and_then(supported_tag) {
        Some(java) => format!("{image}:java{java}"),
        None => format!("{image}:latest"),
    }
}

/// The published image tag that satisfies a minimum Java requirement.
///
/// Rounds *up* to the next available tag, which is the correct direction for a
/// minimum: PaperMC only reports 6, 8, 16, 17, 21 and 25, and the oldest tag
/// published is 8, so a 1.7.10-era requirement of 6 runs on `java8` rather than
/// the floating `latest` (Java 25, which those versions cannot run).
fn supported_tag(minimum: u8) -> Option<u8> {
    const TAGS: [u8; 6] = [8, 11, 16, 17, 21, 25];
    TAGS.into_iter().find(|tag| *tag >= minimum)
}

/// True when the image reference already carries a tag.
fn has_tag(image: &str) -> bool {
    let last_segment = image.rsplit('/').next().unwrap_or(image);
    last_segment.contains(':')
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIN: &str = "/usr/bin/podman";

    fn spec() -> CreateSpec<'static> {
        CreateSpec {
            id: "6f9619ff-816d-4d01-b42d-00cf4fc964ff",
            version: "1.21.8",
            memory_mb: 4096,
            host_port: 25566,
            image: "itzg/minecraft-server:java21",
        }
    }

    #[test]
    fn create_publishes_the_port_and_mounts_a_named_volume() {
        let invocation = create(Path::new(BIN), &spec());

        assert_eq!(invocation.program, PathBuf::from(BIN));
        assert_eq!(
            invocation.args,
            vec![
                "run",
                "--detach",
                "--name",
                "pumapanel-6f9619ff-816d-4d01-b42d-00cf4fc964ff",
                "--label",
                "pumapanel.managed=true",
                "--label",
                "pumapanel.server=6f9619ff-816d-4d01-b42d-00cf4fc964ff",
                "--label",
                "pumapanel.version=1.21.8",
                "--restart",
                "unless-stopped",
                "--env",
                "EULA=TRUE",
                "--env",
                "TYPE=PAPER",
                "--env",
                "VERSION=1.21.8",
                "--env",
                "MEMORY=4096M",
                "--env",
                "INIT_MEMORY=4096M",
                "--publish",
                "25566:25565",
                "--volume",
                "pumapanel-6f9619ff-816d-4d01-b42d-00cf4fc964ff:/data",
                "itzg/minecraft-server:java21",
            ]
        );
    }

    #[test]
    fn lifecycle_commands_target_the_container_name() {
        let id = "abc";
        assert_eq!(
            start(Path::new(BIN), id).args,
            vec!["start", "pumapanel-abc"]
        );
        assert_eq!(
            stop(Path::new(BIN), id).args,
            vec!["stop", "--time", "30", "pumapanel-abc"]
        );
        assert_eq!(
            remove(Path::new(BIN), id).args,
            vec!["rm", "--force", "pumapanel-abc"]
        );
        assert_eq!(
            remove_volume(Path::new(BIN), id).args,
            vec!["volume", "rm", "pumapanel-abc"]
        );
        assert_eq!(
            inspect(Path::new(BIN), &container_name(id)).args,
            vec!["inspect", "pumapanel-abc"]
        );
    }

    #[test]
    fn list_filters_on_the_managed_label() {
        assert_eq!(
            list_managed(Path::new(BIN)).args,
            vec![
                "ps",
                "--all",
                "--filter",
                "label=pumapanel.managed=true",
                "--format",
                "{{.Names}}",
            ]
        );
    }

    #[test]
    fn image_tag_follows_the_minimum_java() {
        assert_eq!(
            image_for_java("itzg/minecraft-server", Some(21)),
            "itzg/minecraft-server:java21"
        );
        assert_eq!(
            image_for_java("itzg/minecraft-server", Some(17)),
            "itzg/minecraft-server:java17"
        );
        assert_eq!(
            image_for_java("itzg/minecraft-server", Some(25)),
            "itzg/minecraft-server:java25"
        );
        // A requirement below the oldest published tag still lands on it, so
        // 1.7.10-era versions get Java 8 rather than the floating tag.
        assert_eq!(
            image_for_java("itzg/minecraft-server", Some(6)),
            "itzg/minecraft-server:java8"
        );
        // A requirement between two tags rounds up to the next one.
        assert_eq!(
            image_for_java("itzg/minecraft-server", Some(19)),
            "itzg/minecraft-server:java21"
        );
        // Beyond the newest tag, and when the minimum is unknown, the floating
        // tag is the only sensible choice.
        assert_eq!(
            image_for_java("itzg/minecraft-server", Some(26)),
            "itzg/minecraft-server:latest"
        );
        assert_eq!(
            image_for_java("itzg/minecraft-server", None),
            "itzg/minecraft-server:latest"
        );
    }

    #[test]
    fn an_explicitly_tagged_image_is_left_alone() {
        assert_eq!(
            image_for_java("registry.internal:5000/minecraft-server:stable-java21", Some(17)),
            "registry.internal:5000/minecraft-server:stable-java21"
        );
    }

    #[test]
    fn ids_must_be_name_safe() {
        assert!(is_valid_id("6f9619ff-816d-4d01-b42d-00cf4fc964ff"));
        assert!(is_valid_id("abc_123.def"));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("has space"));
        assert!(!is_valid_id("../../etc/passwd"));
        assert!(!is_valid_id(&"a".repeat(65)));
    }
}
