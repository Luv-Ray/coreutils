// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (flags) reflink (fs) tmpfs (linux) rlimit Rlim NOFILE clob btrfs neve ROOTDIR USERDIR outfile uufs xattrs
// spell-checker:ignore bdfl hlsl IRWXO IRWXG nconfined matchpathcon libselinux-devel
use uucore::display::Quotable;
use uutests::util::TestScenario;
use uutests::{at_and_ucmd, new_ucmd, path_concat, util_name};

#[cfg(not(windows))]
use std::fs::set_permissions;

use std::io::Write;
#[cfg(not(windows))]
use std::os::unix::fs;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
#[cfg(not(windows))]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(any(target_os = "linux", target_os = "android"))]
use filetime::FileTime;
#[cfg(target_os = "linux")]
use std::ffi::OsString;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs as std_fs;
use std::thread::sleep;
use std::time::Duration;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[cfg(feature = "truncate")]
use uutests::util::PATH;

static TEST_EXISTING_FILE: &str = "existing_file.txt";
static TEST_HELLO_WORLD_SOURCE: &str = "hello_world.txt";
static TEST_HELLO_WORLD_SOURCE_SYMLINK: &str = "hello_world.txt.link";
static TEST_HELLO_WORLD_DEST: &str = "copy_of_hello_world.txt";
static TEST_HELLO_WORLD_DEST_SYMLINK: &str = "copy_of_hello_world.txt.link";
static TEST_HOW_ARE_YOU_SOURCE: &str = "how_are_you.txt";
static TEST_HOW_ARE_YOU_DEST: &str = "hello_dir/how_are_you.txt";
static TEST_COPY_TO_FOLDER: &str = "hello_dir/";
static TEST_COPY_TO_FOLDER_FILE: &str = "hello_dir/hello_world.txt";
static TEST_COPY_FROM_FOLDER: &str = "hello_dir_with_file/";
static TEST_COPY_FROM_FOLDER_FILE: &str = "hello_dir_with_file/hello_world.txt";
static TEST_COPY_TO_FOLDER_NEW: &str = "hello_dir_new";
static TEST_COPY_TO_FOLDER_NEW_FILE: &str = "hello_dir_new/hello_world.txt";
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
static TEST_MOUNT_COPY_FROM_FOLDER: &str = "dir_with_mount";
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
static TEST_MOUNT_MOUNTPOINT: &str = "mount";
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
static TEST_MOUNT_OTHER_FILESYSTEM_FILE: &str = "mount/DO_NOT_copy_me.txt";
static TEST_NONEXISTENT_FILE: &str = "nonexistent_file.txt";
#[cfg(all(
    unix,
    not(any(target_os = "android", target_os = "macos", target_os = "openbsd"))
))]
use uutests::util::compare_xattrs;

/// Assert that mode, ownership, and permissions of two metadata objects match.
#[cfg(all(not(windows), not(target_os = "freebsd")))]
macro_rules! assert_metadata_eq {
    ($m1:expr, $m2:expr) => {{
        assert_eq!($m1.mode(), $m2.mode(), "mode is different");
        assert_eq!($m1.uid(), $m2.uid(), "uid is different");
        assert_eq!($m1.atime(), $m2.atime(), "atime is different");
        assert_eq!(
            $m1.atime_nsec(),
            $m2.atime_nsec(),
            "atime_nsec is different"
        );
        assert_eq!($m1.mtime(), $m2.mtime(), "mtime is different");
        assert_eq!(
            $m1.mtime_nsec(),
            $m2.mtime_nsec(),
            "mtime_nsec is different"
        );
    }};
}

/// only unix has `/dev/fd/0`
#[cfg(unix)]
#[test]
fn test_cp_from_stream() {
    let target = "target";
    let test_string1 = "longer: Hello, World!\n";
    let test_string2 = "shorter";
    let scenario = TestScenario::new(util_name!());
    let at = &scenario.fixtures;

    let mut ucmd = scenario.ucmd();
    let res = ucmd
        .arg("/dev/fd/0")
        .arg(target)
        .pipe_in(test_string1)
        .succeeds()
        .no_stdout();
    assert_eq!(at.read(target), test_string1);

    let mut ucmd = scenario.ucmd();
    let res = ucmd
        .arg("/dev/fd/0")
        .arg(target)
        .pipe_in(test_string2)
        .succeeds()
        .no_stdout();
    assert_eq!(at.read(target), test_string2);
}

#[test]
fn test_freebsd() {
    println!(
        "src permission: {:?}",
        std::fs::File::open("/dev/fd/0")
            .unwrap()
            .metadata()
            .unwrap()
            .permissions()
    );

    panic!(
        "{:?}",
        std::fs::File::open("/dev/fd/0")
            .unwrap()
            .metadata()
            .unwrap()
            .permissions()
    );
}

/// only unix has `/dev/fd/0`
#[cfg(unix)]
#[test]
fn test_cp_from_stream1() {
    let target = "target";
    let test_string1 = "longer: Hello, World!\n";
    let test_string2 = "shorter";
    let scenario = TestScenario::new(util_name!());
    let at = &scenario.fixtures;

    let mut ucmd = scenario.ucmd();
    let res = ucmd
        .arg("/dev/fd/0")
        .arg(target)
        .pipe_in(test_string1)
        .succeeds();
    assert_eq!(at.read(target), test_string1);

    let mut ucmd = scenario.ucmd();
    let res = ucmd
        .arg("/dev/fd/0")
        .arg(target)
        .pipe_in(test_string2)
        .fails()
        .no_stdout();
    assert_eq!(at.read(target), test_string2);
}

/// only unix has `/dev/fd/0`
#[cfg(unix)]
#[test]
fn test_cp_from_stream_permission() {
    let target = "target";
    let link = "link";
    let test_string = "Hello, World!\n";
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch(target);
    at.symlink_file(target, link);
    let mode = 0o777;
    at.set_mode("target", mode);

    ucmd.arg("/dev/fd/0")
        .arg(link)
        .pipe_in(test_string)
        .succeeds();

    assert_eq!(at.read(target), test_string);
    assert_eq!(at.metadata(target).permissions().mode(), 0o100_777);
}

#[cfg(feature = "feat_selinux")]
fn get_getfattr_output(f: &str) -> String {
    use std::process::Command;

    let getfattr_output = Command::new("getfattr")
        .arg(f)
        .arg("-n")
        .arg("security.selinux")
        .output()
        .expect("Failed to run `getfattr` on the destination file");
    println!("{:?}", getfattr_output);
    assert!(
        getfattr_output.status.success(),
        "getfattr did not run successfully: {}",
        String::from_utf8_lossy(&getfattr_output.stderr)
    );

    String::from_utf8_lossy(&getfattr_output.stdout)
        .split('"')
        .nth(1)
        .unwrap_or("")
        .to_string()
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_cp_selinux() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let args = ["-Z", "--context=unconfined_u:object_r:user_tmp_t:s0"];
    at.touch(TEST_HELLO_WORLD_SOURCE);
    for arg in args {
        ts.ucmd()
            .arg(arg)
            .arg(TEST_HELLO_WORLD_SOURCE)
            .arg(TEST_HELLO_WORLD_DEST)
            .succeeds();
        assert!(at.file_exists(TEST_HELLO_WORLD_DEST));

        let selinux_perm = get_getfattr_output(&at.plus_as_string(TEST_HELLO_WORLD_DEST));

        assert!(
            selinux_perm.contains("unconfined_u"),
            "Expected 'foo' not found in getfattr output:\n{selinux_perm}"
        );
        at.remove(&at.plus_as_string(TEST_HELLO_WORLD_DEST));
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_cp_selinux_invalid() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(TEST_HELLO_WORLD_SOURCE);
    let args = [
        "--context=a",
        "--context=unconfined_u:object_r:user_tmp_t:s0:a",
        "--context=nconfined_u:object_r:user_tmp_t:s0",
    ];
    for arg in args {
        new_ucmd!()
            .arg(arg)
            .arg(TEST_HELLO_WORLD_SOURCE)
            .arg(TEST_HELLO_WORLD_DEST)
            .fails()
            .stderr_contains("failed to");
        if at.file_exists(TEST_HELLO_WORLD_DEST) {
            at.remove(TEST_HELLO_WORLD_DEST);
        }
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_cp_preserve_selinux() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let args = ["-Z", "--context=unconfined_u:object_r:user_tmp_t:s0"];
    at.touch(TEST_HELLO_WORLD_SOURCE);
    for arg in args {
        ts.ucmd()
            .arg(arg)
            .arg(TEST_HELLO_WORLD_SOURCE)
            .arg(TEST_HELLO_WORLD_DEST)
            .arg("--preserve=all")
            .succeeds();
        assert!(at.file_exists(TEST_HELLO_WORLD_DEST));
        let selinux_perm_dest = get_getfattr_output(&at.plus_as_string(TEST_HELLO_WORLD_DEST));
        assert!(
            selinux_perm_dest.contains("unconfined_u"),
            "Expected 'foo' not found in getfattr output:\n{selinux_perm_dest}"
        );
        assert_eq!(
            get_getfattr_output(&at.plus_as_string(TEST_HELLO_WORLD_SOURCE)),
            selinux_perm_dest
        );

        #[cfg(all(unix, not(target_os = "freebsd")))]
        {
            // Assert that the mode, ownership, and timestamps are preserved
            // NOTICE: the ownership is not modified on the src file, because that requires root permissions
            let metadata_src = at.metadata(TEST_HELLO_WORLD_SOURCE);
            let metadata_dst = at.metadata(TEST_HELLO_WORLD_DEST);
            assert_metadata_eq!(metadata_src, metadata_dst);
        }

        at.remove(&at.plus_as_string(TEST_HELLO_WORLD_DEST));
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_cp_preserve_selinux_admin_context() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.touch(TEST_HELLO_WORLD_SOURCE);

    // Get the default SELinux context for the destination file path
    // On Debian/Ubuntu, this program is provided by the selinux-utils package
    // On Fedora/RHEL, this program is provided by the libselinux-devel package
    let output = std::process::Command::new("matchpathcon")
        .arg(at.plus_as_string(TEST_HELLO_WORLD_DEST))
        .output()
        .expect("failed to execute matchpathcon command");

    assert!(
        output.status.success(),
        "matchpathcon command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_str = String::from_utf8_lossy(&output.stdout);
    let default_context = output_str
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_string();

    assert!(
        !default_context.is_empty(),
        "Unable to determine default SELinux context for the test file"
    );

    let cmd_result = ts
        .ucmd()
        .arg("-Z")
        .arg(format!("--context={}", default_context))
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    println!("cp command result: {:?}", cmd_result);

    if !cmd_result.succeeded() {
        println!("Skipping test: Cannot set SELinux context, system may not support this context");
        return;
    }

    assert!(at.file_exists(TEST_HELLO_WORLD_DEST));

    let selinux_perm_dest = get_getfattr_output(&at.plus_as_string(TEST_HELLO_WORLD_DEST));
    println!("Destination SELinux context: {}", selinux_perm_dest);

    assert_eq!(default_context, selinux_perm_dest);

    at.remove(&at.plus_as_string(TEST_HELLO_WORLD_DEST));
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_cp_selinux_context_priority() {
    // This test verifies that the priority order is respected:
    // -Z > --context > --preserve=context

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.write(TEST_HELLO_WORLD_SOURCE, "source content");

    // First, set a known context on source file (only if system supports it)
    let setup_result = ts
        .ucmd()
        .arg("--context=unconfined_u:object_r:user_tmp_t:s0")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("initial_context.txt")
        .run();

    // If the system doesn't support setting contexts, skip the test
    if !setup_result.succeeded() {
        println!("Skipping test: System doesn't support setting SELinux contexts");
        return;
    }

    // Create different copies with different context options

    // 1. Using --preserve=context
    ts.ucmd()
        .arg("--preserve=context")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("preserve.txt")
        .succeeds();

    // 2. Using --context with a different context (we already know this works from setup)
    ts.ucmd()
        .arg("--context=unconfined_u:object_r:user_tmp_t:s0")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("context.txt")
        .succeeds();

    // 3. Using -Z (should use default type context)
    ts.ucmd()
        .arg("-Z")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("z_flag.txt")
        .succeeds();

    // 4. Using both -Z and --context (Z should win)
    ts.ucmd()
        .arg("-Z")
        .arg("--context=unconfined_u:object_r:user_tmp_t:s0")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("z_and_context.txt")
        .succeeds();

    // 5. Using both -Z and --preserve=context (Z should win)
    ts.ucmd()
        .arg("-Z")
        .arg("--preserve=context")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("z_and_preserve.txt")
        .succeeds();

    // Get all the contexts
    let source_ctx = get_getfattr_output(&at.plus_as_string(TEST_HELLO_WORLD_SOURCE));
    let preserve_ctx = get_getfattr_output(&at.plus_as_string("preserve.txt"));
    let context_ctx = get_getfattr_output(&at.plus_as_string("context.txt"));
    let z_ctx = get_getfattr_output(&at.plus_as_string("z_flag.txt"));
    let z_and_context_ctx = get_getfattr_output(&at.plus_as_string("z_and_context.txt"));
    let z_and_preserve_ctx = get_getfattr_output(&at.plus_as_string("z_and_preserve.txt"));

    if source_ctx.is_empty() {
        println!("Skipping test assertions: Failed to get SELinux contexts");
        return;
    }
    assert_eq!(
        source_ctx, preserve_ctx,
        "--preserve=context should match the source context"
    );
    assert_eq!(
        source_ctx, context_ctx,
        "--preserve=context should match the source context"
    );
    assert_eq!(
        z_ctx, z_and_context_ctx,
        "-Z context should be the same regardless of --context"
    );
    assert_eq!(
        z_ctx, z_and_preserve_ctx,
        "-Z context should be the same regardless of --preserve=context"
    );
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_cp_selinux_empty_context() {
    // This test verifies that --context without a value works like -Z

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.write(TEST_HELLO_WORLD_SOURCE, "test content");

    // Try creating copies - if this fails, the system doesn't support SELinux properly
    let z_result = ts
        .ucmd()
        .arg("-Z")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("z_flag.txt")
        .run();

    if !z_result.succeeded() {
        println!("Skipping test: SELinux contexts not supported");
        return;
    }

    // Now try with --context (no value)
    let context_result = ts
        .ucmd()
        .arg("--context")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("empty_context.txt")
        .run();

    if !context_result.succeeded() {
        println!("Skipping test: Empty context parameter not supported");
        return;
    }

    let z_ctx = get_getfattr_output(&at.plus_as_string("z_flag.txt"));
    let empty_ctx = get_getfattr_output(&at.plus_as_string("empty_context.txt"));

    if !z_ctx.is_empty() && !empty_ctx.is_empty() {
        assert_eq!(
            z_ctx, empty_ctx,
            "--context without a value should behave like -Z"
        );
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_cp_selinux_recursive() {
    // Test SELinux context preservation in recursive directory copies

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir("source_dir");
    at.write("source_dir/file1.txt", "file1 content");
    at.mkdir("source_dir/subdir");
    at.write("source_dir/subdir/file2.txt", "file2 content");

    let setup_result = ts
        .ucmd()
        .arg("--context=unconfined_u:object_r:user_tmp_t:s0")
        .arg("source_dir/file1.txt")
        .arg("source_dir/context_set.txt")
        .run();

    if !setup_result.succeeded() {
        println!("Skipping test: System doesn't support setting SELinux contexts");
        return;
    }

    ts.ucmd()
        .arg("-rZ")
        .arg("source_dir")
        .arg("dest_dir_z")
        .succeeds();

    ts.ucmd()
        .arg("-r")
        .arg("--preserve=context")
        .arg("source_dir")
        .arg("dest_dir_preserve")
        .succeeds();

    let z_dir_ctx = get_getfattr_output(&at.plus_as_string("dest_dir_z"));
    let preserve_dir_ctx = get_getfattr_output(&at.plus_as_string("dest_dir_preserve"));

    if !z_dir_ctx.is_empty() && !preserve_dir_ctx.is_empty() {
        assert!(
            z_dir_ctx.contains("_u:"),
            "SELinux contexts not properly set with -Z flag"
        );

        assert!(
            preserve_dir_ctx.contains("_u:"),
            "SELinux contexts not properly preserved with --preserve=context"
        );
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_cp_preserve_context_root() {
    use uutests::util::run_ucmd_as_root;
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source_file = "c";
    let dest_file = "e";
    at.touch(source_file);

    let context = "root:object_r:tmp_t:s0";

    let chcon_result = std::process::Command::new("chcon")
        .arg(context)
        .arg(at.plus_as_string(source_file))
        .status();

    if !chcon_result.is_ok_and(|status| status.success()) {
        println!("Skipping test: Failed to set context: {}", context);
        return;
    }

    // Copy the file with preserved context
    // Only works as root
    if let Ok(result) = run_ucmd_as_root(&scene, &["--preserve=context", source_file, dest_file]) {
        let src_ctx = get_getfattr_output(&at.plus_as_string(source_file));
        let dest_ctx = get_getfattr_output(&at.plus_as_string(dest_file));
        println!("Source context: {}", src_ctx);
        println!("Destination context: {}", dest_ctx);

        if !result.succeeded() {
            println!("Skipping test: Failed to copy with preserved context");
            return;
        }

        let dest_context = get_getfattr_output(&at.plus_as_string(dest_file));

        assert!(
            dest_context.contains("root:object_r:tmp_t"),
            "Expected context '{}' not found in destination context: '{}'",
            context,
            dest_context
        );
    } else {
        print!("Test skipped; requires root user");
    }
}
