//! One typed admission for every transaction that targets a single Brain.
//!
//! Canonical order, so every surface answers the same way:
//!
//! 0. Transport (browser, device or either): no database.
//! 1. Principal: account, device and, when asked, the browser session (401).
//! 2. Visibility: a Brain the caller cannot see is 404 *before* any lock is
//!    taken, so locking never becomes an existence oracle. A deleted Brain has
//!    no row and is also 404.
//! 3. Role floor (403).
//! 4. Archive fence (409).
//! 5. Surface-specific locks and checks stay with the surface (for example the
//!    MCP catalogue advisory lock).
//!
//! Steps 2 to 4 cost one round trip (plus one for a requested timeout, which
//! must precede the lock wait it bounds): `recollect_admit_brain` (migration 070)
//! locks, then reads the role and archive flag in separate statements so both
//! reflect whatever committed while this transaction waited for the lock. The
//! lock namespace and modes are exactly those of `recollect_lock_brain`.
//! [`decide`] holds the whole policy as a pure function.

use crate::{
    auth::Auth,
    db,
    error::{Error, Result},
};
use axum::http::StatusCode;
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Duration;
use uuid::Uuid;

pub type Tx<'a> = Transaction<'a, Postgres>;

/// Effective Brain role. The derived order is the authority order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Reader,
    Writer,
    Admin,
}

impl Role {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "reader" => Some(Self::Reader),
            "writer" => Some(Self::Writer),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reader => "reader",
            Self::Writer => "writer",
            Self::Admin => "admin",
        }
    }
    pub fn is_admin(self) -> bool {
        self == Self::Admin
    }
}

/// Brain admission lock: queued shared admission or the exclusive writer lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lock {
    Shared,
    Exclusive,
}

/// Which credential may use the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    Any,
    Browser,
    Device,
}

/// What an archived Brain means to the surface. `Reject` and `AdminBrowserOnly`
/// carry the surface's own error so its code and message stay stable.
#[derive(Clone, Copy, Debug)]
pub enum Archived {
    /// The surface works on archived Brains.
    Allow,
    /// Archived Brains are refused with this error.
    Reject(fn() -> Error),
    /// Archived Brains stay readable for a browser administrator only; every
    /// other caller gets this error.
    AdminBrowserOnly(fn() -> Error),
}

/// What a surface requires of the caller and the Brain.
#[derive(Clone, Copy, Debug)]
pub struct BrainAccess {
    pub role: Role,
    pub lock: Lock,
    pub transport: Transport,
    pub archived: Archived,
    /// Also lock and re-check the browser session (devices are always locked
    /// and checked). Ignored by the worker variants, which have no session.
    pub recheck_session: bool,
    /// `SET LOCAL lock_timeout` before the lock wait, so a busy Brain fails fast
    /// instead of pinning the connection.
    pub lock_timeout: Option<Duration>,
    /// `SET LOCAL statement_timeout` before the lock wait, bounding it and every
    /// later statement of the transaction.
    pub statement_timeout: Option<Duration>,
}

impl BrainAccess {
    /// Any member, queued shared lock, any transport, archived Brains allowed.
    pub const READ: Self = Self {
        role: Role::Reader,
        lock: Lock::Shared,
        transport: Transport::Any,
        archived: Archived::Allow,
        recheck_session: false,
        lock_timeout: None,
        statement_timeout: None,
    };
    /// Writer or administrator, exclusive lock, archived Brains refused with the
    /// evidence error.
    pub const WRITE: Self = Self {
        role: Role::Writer,
        lock: Lock::Exclusive,
        transport: Transport::Any,
        archived: Archived::Reject(evidence_archived),
        recheck_session: false,
        lock_timeout: None,
        statement_timeout: None,
    };
    /// Administrator under the exclusive lock. The archive policy has no default.
    pub const fn admin(archived: Archived) -> Self {
        Self {
            role: Role::Admin,
            lock: Lock::Exclusive,
            transport: Transport::Any,
            archived,
            recheck_session: false,
            lock_timeout: None,
            statement_timeout: None,
        }
    }
    pub const fn with_role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }
    pub const fn with_lock(mut self, lock: Lock) -> Self {
        self.lock = lock;
        self
    }
    pub const fn with_transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }
    pub const fn with_archived(mut self, archived: Archived) -> Self {
        self.archived = archived;
        self
    }
    pub const fn with_session_recheck(mut self) -> Self {
        self.recheck_session = true;
        self
    }
    pub const fn with_lock_timeout(mut self, timeout: Duration) -> Self {
        self.lock_timeout = Some(timeout);
        self
    }
    pub const fn with_statement_timeout(mut self, timeout: Duration) -> Self {
        self.statement_timeout = Some(timeout);
        self
    }
}

/// An open transaction that passed admission, with what admission learned.
pub struct Admitted<'a> {
    pub tx: Tx<'a>,
    pub role: Role,
    pub archived: bool,
}

/// Evidence mutations on an archived Brain (the existing `require_writer` error).
pub fn evidence_archived() -> Error {
    Error(
        StatusCode::CONFLICT,
        "brain_archived",
        "Reopen this Brain before changing its evidence.",
    )
}

/// The existing `Auth::require_browser` error.
pub fn browser_required() -> Error {
    Error(
        StatusCode::FORBIDDEN,
        "browser_required",
        "Use a signed-in browser for this action.",
    )
}

/// A surface that only a paired device may use.
pub fn device_required() -> Error {
    Error(
        StatusCode::FORBIDDEN,
        "device_required",
        "This endpoint requires a paired device.",
    )
}

/// Step 0 of the canonical order: no database.
pub fn transport(access: &BrainAccess, device: bool) -> Result<()> {
    match access.transport {
        Transport::Browser if device => Err(browser_required()),
        Transport::Device if !device => Err(device_required()),
        _ => Ok(()),
    }
}

/// The whole policy after the principal is known: transport, then visibility
/// (`role` is `None` for a Brain the caller cannot see), role floor, archive
/// fence, in that order.
pub fn decide(
    access: &BrainAccess,
    device: bool,
    role: Option<Role>,
    archived: bool,
) -> Result<()> {
    transport(access, device)?;
    let role = role.ok_or_else(Error::missing)?;
    if role < access.role {
        return Err(Error::forbidden());
    }
    if archived {
        match access.archived {
            Archived::Allow => {}
            Archived::Reject(error) => return Err(error()),
            Archived::AdminBrowserOnly(error) => {
                if device || !role.is_admin() {
                    return Err(error());
                }
            }
        }
    }
    Ok(())
}

/// Authenticate, then admit the caller to one Brain in a single statement.
pub async fn brain_tx<'a>(
    pool: &'a PgPool,
    auth: &Auth,
    brain: Uuid,
    access: BrainAccess,
) -> Result<Admitted<'a>> {
    let device = auth.device_id.is_some();
    transport(&access, device)?;
    let tx = if access.recheck_session {
        auth.publication_tx(pool).await?
    } else {
        auth.tx(pool).await?
    };
    admit(tx, brain, &access, access.lock, device).await
}

/// Worker variant: the principal is a stored actor and optional device.
pub async fn brain_tx_as<'a>(
    pool: &'a PgPool,
    actor: Uuid,
    device: Option<Uuid>,
    brain: Uuid,
    access: BrainAccess,
) -> Result<Admitted<'a>> {
    transport(&access, device.is_some())?;
    let tx = db::device_tx(pool, actor, device).await?;
    admit(tx, brain, &access, access.lock, device.is_some()).await
}

/// Provisional read-only snapshot (REPEATABLE READ) that checks the same
/// access without locking, matching [`db::preparation_tx`]. Preparation never
/// takes the Brain lock, so the access lock is ignored.
pub async fn brain_preparation_tx<'a>(
    pool: &'a PgPool,
    actor: Uuid,
    device: Option<Uuid>,
    brain: Uuid,
    access: BrainAccess,
) -> Result<Admitted<'a>> {
    transport(&access, device.is_some())?;
    let tx = db::preparation_tx(pool, actor, device).await?;
    admit(tx, brain, &access, Lock::Shared, device.is_some()).await
}

async fn admit<'a>(
    mut tx: Tx<'a>,
    brain: Uuid,
    access: &BrainAccess,
    lock: Lock,
    device: bool,
) -> Result<Admitted<'a>> {
    apply_timeouts(&mut tx, access).await?;
    let (role, archived) = lookup(&mut tx, brain, lock).await?;
    decide(access, device, role, archived)?;
    Ok(Admitted {
        tx,
        role: role.ok_or_else(Error::missing)?,
        archived,
    })
}

/// Admit within a transaction the caller already opened and authenticated, for
/// helpers that receive one. Same single statement, lock and policy as
/// [`brain_tx`]; the transport is not known here, so only `Transport::Any` and
/// archive policies that ignore the credential are accepted. Returns the
/// effective role. A timeout on `access` is ignored: set it before this call.
pub async fn admit_in(tx: &mut Tx<'_>, brain: Uuid, access: &BrainAccess) -> Result<Role> {
    debug_assert!(
        matches!(access.transport, Transport::Any)
            && !matches!(access.archived, Archived::AdminBrowserOnly(_)),
        "admit_in cannot see the credential; open the transaction with brain_tx"
    );
    let (role, archived) = lookup(tx, brain, access.lock).await?;
    decide(access, false, role, archived)?;
    role.ok_or_else(Error::missing)
}

async fn apply_timeouts(tx: &mut Tx<'_>, access: &BrainAccess) -> Result<()> {
    let (mut names, mut values) = (vec![], vec![]);
    for (name, timeout) in [
        ("statement_timeout", access.statement_timeout),
        ("lock_timeout", access.lock_timeout),
    ] {
        if let Some(timeout) = timeout {
            names.push(name);
            values.push(format!("{}ms", timeout.as_millis()));
        }
    }
    if names.is_empty() {
        return Ok(());
    }
    // `set_config(..., true)` is `SET LOCAL`; one statement sets every timeout.
    sqlx::query("SELECT set_config(n,v,true) FROM unnest($1::text[],$2::text[]) AS t(n,v)")
        .bind(names)
        .bind(values)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn lookup(tx: &mut Tx<'_>, brain: Uuid, lock: Lock) -> Result<(Option<Role>, bool)> {
    let (role, archived): (Option<String>, Option<bool>) =
        sqlx::query_as("SELECT role, archived FROM recollect_admit_brain($1,$2)")
            .bind(brain)
            .bind(lock == Lock::Exclusive)
            .fetch_one(&mut **tx)
            .await?;
    // An unknown role name cannot come from the database; it fails closed.
    Ok((
        role.as_deref().and_then(Role::parse),
        archived.unwrap_or(false),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use recollect_protocol::User;

    fn probe() -> Error {
        Error(StatusCode::CONFLICT, "probe_archived", "probe")
    }
    fn code(result: Result<()>) -> Option<(u16, &'static str)> {
        result
            .err()
            .map(|Error(status, code, _)| (status.as_u16(), code))
    }
    const NOT_FOUND: Option<(u16, &str)> = Some((404, "not_found"));
    const FORBIDDEN: Option<(u16, &str)> = Some((403, "forbidden"));
    const ARCHIVED: Option<(u16, &str)> = Some((409, "probe_archived"));

    #[test]
    fn roles_are_ordered_and_parse_exactly() {
        assert!(Role::Reader < Role::Writer && Role::Writer < Role::Admin);
        for role in [Role::Reader, Role::Writer, Role::Admin] {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
        assert_eq!(Role::parse("Admin"), None);
        assert_eq!(Role::parse(""), None);
        assert!(Role::Admin.is_admin() && !Role::Writer.is_admin());
    }

    #[test]
    fn transport_is_checked_without_a_role() {
        let browser = BrainAccess::READ.with_transport(Transport::Browser);
        let device = BrainAccess::READ.with_transport(Transport::Device);
        let any = BrainAccess::READ;
        assert_eq!(code(transport(&any, false)), None);
        assert_eq!(code(transport(&any, true)), None);
        assert_eq!(code(transport(&browser, false)), None);
        assert_eq!(
            code(transport(&browser, true)),
            Some((403, "browser_required"))
        );
        assert_eq!(code(transport(&device, true)), None);
        assert_eq!(
            code(transport(&device, false)),
            Some((403, "device_required"))
        );
    }

    #[test]
    fn invisible_brain_is_not_found_for_every_floor() {
        for access in [
            BrainAccess::READ,
            BrainAccess::WRITE,
            BrainAccess::admin(Archived::Allow),
        ] {
            for device in [false, true] {
                for archived in [false, true] {
                    assert_eq!(code(decide(&access, device, None, archived)), NOT_FOUND);
                }
            }
        }
    }

    #[test]
    fn role_floor_is_forbidden_below_and_open_at_or_above() {
        let archived = Archived::Reject(probe);
        for (floor, allowed) in [
            (Role::Reader, [true, true, true]),
            (Role::Writer, [false, true, true]),
            (Role::Admin, [false, false, true]),
        ] {
            let access = BrainAccess::READ.with_role(floor).with_archived(archived);
            for (role, ok) in [Role::Reader, Role::Writer, Role::Admin]
                .into_iter()
                .zip(allowed)
            {
                let expected = if ok { None } else { FORBIDDEN };
                assert_eq!(code(decide(&access, false, Some(role), false)), expected);
                assert_eq!(code(decide(&access, true, Some(role), false)), expected);
            }
        }
    }

    #[test]
    fn archive_policy_allow_reject_and_admin_browser_only() {
        let allow = BrainAccess::READ.with_archived(Archived::Allow);
        let reject = BrainAccess::READ.with_archived(Archived::Reject(probe));
        let admin_only = BrainAccess::READ.with_archived(Archived::AdminBrowserOnly(probe));
        for role in [Role::Reader, Role::Writer, Role::Admin] {
            for device in [false, true] {
                // An open Brain never trips the fence.
                for access in [&allow, &reject, &admin_only] {
                    assert_eq!(code(decide(access, device, Some(role), false)), None);
                }
                assert_eq!(code(decide(&allow, device, Some(role), true)), None);
                assert_eq!(code(decide(&reject, device, Some(role), true)), ARCHIVED);
                let admin_browser = role.is_admin() && !device;
                assert_eq!(
                    code(decide(&admin_only, device, Some(role), true)),
                    if admin_browser { None } else { ARCHIVED }
                );
            }
        }
    }

    #[test]
    fn canonical_order_is_transport_then_visibility_then_role_then_archive() {
        let access = BrainAccess::admin(Archived::Reject(probe)).with_transport(Transport::Browser);
        // Transport beats everything, even a Brain that is not visible.
        assert_eq!(
            code(decide(&access, true, None, true)),
            Some((403, "browser_required"))
        );
        // Visibility beats role and archive.
        assert_eq!(code(decide(&access, false, None, true)), NOT_FOUND);
        // Role beats the archive fence: a non-admin on an archived Brain is 403.
        assert_eq!(
            code(decide(&access, false, Some(Role::Writer), true)),
            FORBIDDEN
        );
        assert_eq!(
            code(decide(&access, false, Some(Role::Reader), true)),
            FORBIDDEN
        );
        // Only an administrator reaches the archive fence.
        assert_eq!(
            code(decide(&access, false, Some(Role::Admin), true)),
            ARCHIVED
        );
        assert_eq!(code(decide(&access, false, Some(Role::Admin), false)), None);
    }

    #[test]
    fn constants_match_the_documented_shapes() {
        let read = BrainAccess::READ;
        assert_eq!(
            (read.role, read.lock, read.transport, read.recheck_session),
            (Role::Reader, Lock::Shared, Transport::Any, false)
        );
        assert!(matches!(read.archived, Archived::Allow));
        let write = BrainAccess::WRITE;
        assert_eq!(
            (
                write.role,
                write.lock,
                write.transport,
                write.recheck_session
            ),
            (Role::Writer, Lock::Exclusive, Transport::Any, false)
        );
        // The write default refuses with the evidence error, unchanged.
        let evidence = code(decide(&write, false, Some(Role::Writer), true));
        assert_eq!(evidence, Some((409, "brain_archived")));
        let admin = BrainAccess::admin(Archived::Allow);
        assert_eq!((admin.role, admin.lock), (Role::Admin, Lock::Exclusive));
        assert!(BrainAccess::READ.with_session_recheck().recheck_session);
        assert_eq!(
            BrainAccess::READ.with_lock(Lock::Exclusive).lock,
            Lock::Exclusive
        );
    }

    #[test]
    fn timeouts_are_opt_in_and_independent() {
        for access in [BrainAccess::READ, BrainAccess::WRITE] {
            assert_eq!(
                (access.lock_timeout, access.statement_timeout),
                (None, None)
            );
        }
        let lock = BrainAccess::WRITE.with_lock_timeout(Duration::from_millis(250));
        assert_eq!(lock.lock_timeout, Some(Duration::from_millis(250)));
        assert_eq!(lock.statement_timeout, None);
        let both = lock.with_statement_timeout(Duration::from_secs(2));
        assert_eq!(both.statement_timeout, Some(Duration::from_secs(2)));
        assert_eq!(both.lock_timeout, Some(Duration::from_millis(250)));
        // A timeout changes no policy: the decision is the same.
        let decide_with =
            |access: &BrainAccess| code(decide(access, false, Some(Role::Writer), true));
        assert_eq!(decide_with(&both), decide_with(&BrainAccess::WRITE));
    }

    #[test]
    fn constructors_keep_the_existing_error_texts() {
        let user = User {
            id: Uuid::new_v4(),
            username: "probe".into(),
            installation_owner: false,
        };
        let device = Auth {
            user,
            session: Uuid::nil(),
            csrf: Uuid::nil(),
            device_id: Some(Uuid::new_v4()),
        };
        let existing = device.require_browser().unwrap_err();
        let ours = browser_required();
        assert_eq!(
            (existing.0, existing.1, existing.2),
            (ours.0, ours.1, ours.2)
        );
        // A missing device is no longer the generic forbidden error.
        let required = device_required();
        assert_eq!(
            (required.0, required.1, required.2),
            (
                StatusCode::FORBIDDEN,
                "device_required",
                "This endpoint requires a paired device."
            )
        );
        let evidence = evidence_archived();
        assert_eq!(
            (evidence.0, evidence.1),
            (StatusCode::CONFLICT, "brain_archived")
        );
    }
}
