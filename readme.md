# psx

A lightweight Linux process viewer written in Rust. Reads process information directly from `/proc`, providing CPU usage, memory consumption, execution time, user filtering, keyword searching, sorting, and JSON output.

## Features

- **Lightweight:** No external dependencies; uses Linux `/proc` filesystem.
- **CPU usage:** Displays process CPU utilization as a percentage.
- **Memory usage:** Shows resident memory (RSS) in KB, MB, or GB.
- **Execution time:** Human-readable process CPU time.
- **User filtering:** Display processes belonging to a specific user.
- **Keyword search:** Search process commands with case-insensitive matching and highlighted results.
- **Sorting:** Sort by PID, memory, CPU, or execution time in ascending or descending order.
- **Adaptive output:** Automatically adjusts command column width to terminal size and wraps long commands.
- **Simplified commands:** Displays concise commands by default, with an option to show full command lines.
- **JSON output:** Machine-readable output for scripting and integration.

## Installation

### Build from source

Requires Rust and Cargo.

```bash
git clone https://github.com/aleee-dev1/psx
cd psx
cargo build --release
```

The compiled binary will be available at:

```bash
target/release/psx
```

To install system-wide:

```bash
sudo install -m 755 target/release/psx /usr/local/bin/psx
```

## Usage

```bash
psx [OPTIONS] [KEYWORD]
```

Running `psx` without arguments displays the current processes.

### Options

| Option | Description |
|---|---|
| `-a` | Show all processes and display full command lines |
| `-j` | Output results in JSON format |
| `-u USER` | Filter processes by username |
| `-uUSER` | Same as `-u USER` |
| `-s<KEY><ORDER>` | Configure sorting |
| `KEYWORD` | Filter processes by command name or command line |

### Sorting

The `-s` option accepts two characters:

**Sort key:**

| Key | Sort by |
|---|---|
| `p` | PID |
| `m` | Memory usage |
| `c` | CPU usage |
| `t` | Execution time |

**Sort order:**

| Order | Description |
|---|---|
| `a` | Ascending |
| `d` | Descending |

Default sorting is CPU usage in descending order (`-scd`).

## Examples

**Display processes:**

```bash
psx
```

**Show all processes with full command lines:**

```bash
psx -a
```

**Search for a process:**

```bash
psx firefox
```

**Show processes belonging to a user:**

```bash
psx -u ali
```

**Sort by memory usage (highest first):**

```bash
psx -smd
```

**Sort by CPU usage (lowest first):**

```bash
psx -sca
```

**Sort by PID (ascending):**

```bash
psx -spa
```

**Sort by execution time (highest first):**

```bash
psx -std
```

**Combine filtering and sorting:**

```bash
psx -u ali -smd firefox
```

**Export process information as JSON:**

```bash
psx -j
```

**Show all processes as JSON:**

```bash
psx -a -j
```

## Output

Default terminal output:

```text
PID    TIME       USER  CPU    MEM    COMMAND
1234   2h 15m     ali   12.4%  256mb  firefox
5678   45m 12s    ali    4.2%  128mb  node /home/ali/app.js
9012   12m 30s    root   1.1%   64mb  /usr/lib/systemd/systemd
```

JSON output includes the following fields:

| Field | Type | Description |
|---|---|---|
| `pid` | Integer | Process ID |
| `time_seconds` | Integer | CPU time in seconds |
| `time` | String | Formatted CPU time |
| `user` | String | Process owner |
| `cpu` | Float | CPU usage percentage |
| `mem_kb` | Integer | Resident memory in KB |
| `mem` | String | Formatted memory usage |
| `command` | String | Full command line |

## How it works

`psx` reads process information directly from Linux's `/proc` filesystem:

- `/proc/[pid]/stat` — PID, CPU time, and process start time.
- `/proc/[pid]/status` — Effective UID and resident memory.
- `/proc/[pid]/cmdline` — Full process command line.
- `/proc/uptime` — System uptime.
- `/etc/passwd` — UID-to-username mapping.

CPU utilization is calculated using process CPU time relative to elapsed process time.

Processes that disappear or become inaccessible while being scanned are automatically skipped.

## Requirements

- Linux
- Rust toolchain (for building from source)
- Access to `/proc`

## License

MIT
