# Encrypted backup and recovery

Use the named [installation tooling](installation.md) and the accepted
[recovery contract](../contracts/operations-recovery-drills.md). The
[dated evidence](../mappings/recovery-2026-09-26.md) distinguishes the local drill
from an off-machine production backup or a universal capacity claim.

## Prepare explicit recovery inputs

Install the repository-local age CLI with `python3 scripts/setup-recovery-tools.py`.
Its output identifies the platform-specific directory containing `age` and
`age-keygen`. Generate a private recovery identity with that `age-keygen -o FILE`;
keep an independent recoverable copy. Obtain its public recipient with
`age-keygen -y FILE`. The application needs the public recipient, not that private
decryption identity. Existing OpenSSH `sftp` and Docker/Compose are also required.

Create a private, ignored JSON input file using absolute paths:

```json
{
  "backup_dir": "/absolute/Recollect/.data/backups/team",
  "age_binary": "/absolute/Recollect/.cache/recovery-tools/age-v1.3.2-darwin-arm64/age",
  "recipients": ["PUBLIC_AGE_RECIPIENT"],
  "remote": {
    "host": "backup.example.internal",
    "port": 22,
    "user": "recollect-backup",
    "remote_root": "/recollect",
    "identity_file": "/absolute/private/recovery-sftp-key",
    "known_hosts_file": "/absolute/private/recovery-known-hosts"
  }
}
```

Replace the tool directory with the setup result. The remote root must already
exist and be writable by the selected SFTP account. Pin its host key through a
trusted channel. Configuration disables ambient SSH config, agents, forwarding
and proxies. No password or key contents belong in this JSON. Omit `remote` for
local encrypted backups, whose durability is limited to the selected local disk.

```sh
python3 scripts/recovery.py configure team --file .data/team-recovery-input.json
python3 scripts/recovery.py enable-mirror team
python3 scripts/recovery.py status team
```

Configuration reads the current installation UUID and establishes/checks its
owned remote directory. `enable-mirror` briefly drains this installation,
synchronizes existing deletion/analytics entries, and configures the same mirror
reference for API, worker and operator roles. Only actual successful setup is
reported connected. Remote acknowledgements then become part of erasure completion;
an outage keeps erasure pending while local reads remain denied. Vault is optional
and these commands do not revoke any token or lease.

If the container requires a different hostname, pass `--runtime-host HOST` to
`enable-mirror`; the same pinned known-hosts file must cover that hostname. This
is useful for an owned Docker Desktop fixture, not an automatic trust exception.

## Create, schedule and inspect

```sh
python3 scripts/recovery.py create team
python3 scripts/recovery.py list team
python3 scripts/recovery.py scheduled team
python3 scripts/recovery.py prune team
```

Creation stops only previously running application roles, reconciles privacy,
dumps PostgreSQL and copies required artifacts/account credentials/central receipts.
It encrypts before publishing. PostgreSQL/Neo4j and unrelated installations stay
running. Previously running application roles resume, with backup and resume
outcomes recorded separately. A failed remote upload can be retried with
`python3 scripts/recovery.py transfer team BACKUP_UUID`.

`scheduled` performs creation and pruning with separate outcomes. Install it in
the operator's chosen daily scheduler using an absolute Python/repository path
and a working Docker context; no scheduler is installed automatically. For example,
a daily operator job can run `python3 /absolute/Recollect/scripts/recovery.py
scheduled team`. Monitor its exit status and `status`, including checkpoint age,
local/remote state, expiry, last attempt and last prune. The 24-hour recovery-point
objective applies only while that daily schedule and storage are healthy.

An archive expires at the shortest included Brain backup duration (default seven
days, supported one to 365); a shorter current policy can expire it sooner. Pruning
removes only owned expired complete/abandoned partial archives and retains a
content-free status record. Independent deletion journals are never pruned by
this command. Copies made outside this operator's storage remain the operator's
responsibility. After an interrupted backup, inspect `status` before using
`resume-source NAME` to resume the roles that operation stopped.

## Restore after source-machine loss

Preserve the source installation UUID, backup UUID, selected compatible image,
operator keys and runtime credential references independently of its database.
Stop the original and any previously restored writers before the final journal
capture. The tool checks locally recorded installations on its Docker endpoint;
cutover on another machine remains the operator's responsibility.

Initialize a fresh destination with its own unused name/port (or shared HTTPS
inputs), select the compatible application image and configure required providers
and approved MCP executables. Then configure the known source UUID without
contacting its database:

```sh
python3 scripts/install.py init recovered personal --port 8789
python3 scripts/recovery.py configure recovered \
  --file .data/recovered-recovery-input.json --installation-id SOURCE_UUID
python3 scripts/recovery.py fetch recovered BACKUP_UUID --identity /private/age-key
python3 scripts/recovery.py fetch-journals recovered --identity /private/age-key
```

Choose a fresh local backup directory in that input file. `fetch` preserves existing
local archive identities. `fetch-journals` returns the independent encrypted journal
filename; use that current file, never the journal bundled with an old checkpoint.
Alternatively, `export-journals SOURCE` captures the current local journals while
the selected source is still available. Losing every current journal copy blocks
recovery; an older data backup cannot reconstruct later acknowledged deletions.

```sh
python3 scripts/recovery.py restore recovered \
  --config .data/install/recovered/recovery.json \
  --archive /absolute/backups/BACKUP_UUID.tar.age \
  --journals /absolute/backups/journal-CURRENT_UUID.tar.age \
  --identity /private/age-key
python3 scripts/install.py diagnose recovered
```

Supply `--runtime-host HOST` when needed with matching pinned trust. A source's
remote deletion requirement is restored using the supplied current SSH references
before preparation/release; SSH credentials are not taken from the data archive.
Destination database, graph, origin and provider inputs remain explicit operator
configuration. Ordinary local source recovery does not require a remote mirror.

Authentication, archive/schema/artifact/journal validation precede target data
creation. Existing destination volumes are refused. A durable hold prevents normal
startup through import and privacy reconciliation. If interrupted, inspect status
and repeat the exact restore command with `--resume`. It retires only helpers and
import sessions tagged to that restore and removes its marked abandoned staging.
Do not delete the hold or manually rerun old queued work. A failed upgrade is
recovered into another fresh compatible installation, never by reverse migrations
or overwriting the failed installation's volumes.

## Verify the recovered product

Check readiness and graph rebuild completion, then actual permitted recall, source
content, claim/history and scoped graph queries. Pair those with erased/rejected
content checks. The original canonical installation identity is retained, so leave
previous writers stopped. Invited account credentials and account identities are
restored. The bootstrap owner's login still uses the destination's explicit
`RECOLLECT_OWNER_USERNAME`/`RECOLLECT_OWNER_PASSWORD`; preserve that owner's username
and use its current operator-configured password when checking the destination.

Outstanding MCP calls remain uncertain; receipt-only reconciliation can recover
evidence without repeating an effect. Outstanding model requests retain accounting
and are not rebilled automatically. Neo4j rebuilds from eligible canonical data;
old analytics jobs are not blindly replayed. Ordinary changes after the chosen
checkpoint, missing outside sources and lost provider credentials cannot be
invented by restore. The local fifteen-minute acceptance target covers only the
recorded small corpus and excludes image build and operator setup.
