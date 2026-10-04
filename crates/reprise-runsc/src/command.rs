//! Pure command construction, separate from subprocess execution and stdio.
//!
//! Container IDs here are already validated by the public interface; tests use
//! literal IDs so argument-contract failures remain independent of validation.

#![expect(dead_code, reason = "public command methods are not wired yet")]

use std::{ffi::OsString, path::Path};

use crate::{
    CheckpointOptions, Compression, CreateOptions, DeleteOptions, FileAccess, Invocation, Network,
    Operation, Overlay, OverlayBacking, Platform, RestoreOptions, RunscConfig,
};

#[derive(Debug)]
pub(crate) enum Command<'a> {
    Version,
    Create {
        id: &'a str,
        options: &'a CreateOptions,
    },
    Start {
        id: &'a str,
    },
    Run {
        id: &'a str,
        options: &'a CreateOptions,
        detached: bool,
    },
    Checkpoint {
        id: &'a str,
        options: &'a CheckpointOptions,
    },
    Restore {
        id: &'a str,
        options: &'a RestoreOptions,
    },
    State {
        id: &'a str,
    },
    List,
    Wait {
        id: &'a str,
    },
    Delete {
        id: &'a str,
        options: DeleteOptions,
    },
}

/// Builds the runsc invocation in a stable order: global flags, subcommand,
/// operation flags, then the container ID. Paths remain native OS strings so
/// non-UTF-8 paths are passed to the child without loss.
pub(crate) fn invocation(config: &RunscConfig, command: Command<'_>) -> Invocation {
    let mut args = Vec::new();
    push_global_options(&mut args, config);

    let operation = match command {
        Command::Version => {
            args.push("--version".into());
            Operation::Version
        }
        Command::Create { id, options } => {
            args.push("create".into());
            push_create_options(&mut args, options);
            push_id(&mut args, id);
            Operation::Create
        }
        Command::Start { id } => {
            args.push("start".into());
            push_id(&mut args, id);
            Operation::Start
        }
        Command::Run {
            id,
            options,
            detached,
        } => {
            args.push("run".into());
            push_create_options(&mut args, options);
            push_flag(&mut args, "--detach", detached);
            push_id(&mut args, id);
            Operation::Run
        }
        Command::Checkpoint { id, options } => {
            args.push("checkpoint".into());
            push_path_option(&mut args, "--image-path", &options.image_path);
            push_flag(&mut args, "--leave-running", options.leave_running);
            if let Some(compression) = options.compression {
                let value = match compression {
                    Compression::None => "none",
                    Compression::FlateBestSpeed => "flate-best-speed",
                };
                args.push(format!("--compression={value}").into());
            }
            push_flag(
                &mut args,
                "--exclude-committed-zero-pages",
                options.exclude_committed_zero_pages,
            );
            push_flag(&mut args, "--direct", options.direct_io);
            push_id(&mut args, id);
            Operation::Checkpoint
        }
        Command::Restore { id, options } => {
            args.push("restore".into());
            push_create_options(&mut args, &options.create);
            push_path_option(&mut args, "--image-path", &options.image_path);
            // runsc restore always detaches; background and direct I/O only add flags.
            args.push("--detach".into());
            push_flag(&mut args, "--background", options.background);
            push_flag(&mut args, "--direct", options.direct_io);
            push_id(&mut args, id);
            Operation::Restore
        }
        Command::State { id } => {
            args.push("state".into());
            push_id(&mut args, id);
            Operation::State
        }
        Command::List => {
            args.extend(["list".into(), "--quiet".into()]);
            Operation::List
        }
        Command::Wait { id } => {
            args.push("wait".into());
            push_id(&mut args, id);
            Operation::Wait
        }
        Command::Delete { id, options } => {
            args.push("delete".into());
            push_flag(&mut args, "--force", options.force);
            push_id(&mut args, id);
            Operation::Delete
        }
    };

    Invocation {
        executable: config.executable.clone(),
        operation,
        args,
    }
}

/// runsc parses global settings before the operation, so this order is shared
/// by every command and remains visible in the resulting argv snapshot.
fn push_global_options(args: &mut Vec<OsString>, config: &RunscConfig) {
    args.extend(["--root".into(), config.state_root.as_os_str().to_owned()]);

    let options = &config.options;
    if let Some(platform) = options.platform {
        let value = match platform {
            Platform::Systrap => "systrap",
            Platform::Kvm => "kvm",
        };
        args.push(format!("--platform={value}").into());
    }
    if let Some(network) = options.network {
        let value = match network {
            Network::Sandbox => "sandbox",
            Network::None => "none",
            Network::Host => "host",
        };
        args.push(format!("--network={value}").into());
    }
    if let Some(overlay) = &options.overlay {
        push_overlay_option(args, overlay);
    }
    if let Some(file_access) = options.file_access_mounts {
        let value = match file_access {
            FileAccess::Shared => "shared",
            FileAccess::Exclusive => "exclusive",
        };
        args.push(format!("--file-access-mounts={value}").into());
    }
    if let Some(directfs) = options.directfs {
        args.push(format!("--directfs={directfs}").into());
    }
}

/// Directory overlays embed a path in a single flag, so append its native bytes
/// to the flag prefix instead of formatting it through a lossy UTF-8 string.
fn push_overlay_option(args: &mut Vec<OsString>, overlay: &Overlay) {
    let mut argument = OsString::from("--overlay2=");
    match overlay {
        Overlay::None => argument.push("none"),
        Overlay::Root(backing) => push_overlay_backing(&mut argument, "root", backing),
        Overlay::All(backing) => push_overlay_backing(&mut argument, "all", backing),
    }
    args.push(argument);
}

fn push_overlay_backing(argument: &mut OsString, scope: &str, backing: &OverlayBacking) {
    argument.push(scope);
    argument.push(":");
    match backing {
        OverlayBacking::Memory => argument.push("memory"),
        OverlayBacking::SelfBacked => argument.push("self"),
        OverlayBacking::Directory(path) => {
            argument.push("dir=");
            argument.push(path.as_os_str());
        }
    }
}

/// Bundle and optional PID file are the launch fields shared by create, run,
/// and restore. Stdio stays out of argv and is handled by process execution.
fn push_create_options(args: &mut Vec<OsString>, options: &CreateOptions) {
    push_path_option(args, "--bundle", &options.bundle);
    if let Some(pid_file) = &options.pid_file {
        push_path_option(args, "--pid-file", pid_file);
    }
}

fn push_path_option(args: &mut Vec<OsString>, flag: &str, path: &Path) {
    args.push(flag.into());
    args.push(path.as_os_str().to_owned());
}

fn push_flag(args: &mut Vec<OsString>, flag: &str, enabled: bool) {
    if enabled {
        args.push(flag.into());
    }
}

fn push_id(args: &mut Vec<OsString>, id: &str) {
    args.push(id.into());
}

#[cfg(test)]
mod tests;
