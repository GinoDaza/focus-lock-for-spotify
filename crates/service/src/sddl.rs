//! Editing of DACLs in the security descriptor string format (SDDL).
//!
//! The service protects files by adding two explicit ACEs to their DACL
//! and removing them again later. Doing that on the string form keeps the
//! logic pure and testable; `windows::acl` only reads and writes strings.

/// Denies interactive users every right that changes a file or folder:
/// write data, append, write extended attributes and attributes, delete,
/// delete children, change permissions and take ownership. Inherited by
/// everything inside.
const DENY_CHANGES: &str = "(D;OICI;0xd0156;;;IU)";

/// An ACE for OWNER RIGHTS replaces the implicit right of an owner to
/// rewrite the DACL. This one only lets the owner read it.
const OWNER_READS: &str = "(A;OICI;RC;;;OW)";

/// Adds the protection ACEs to a DACL-only SDDL string (`D:...`). Leaves a
/// string that already has them unchanged.
#[must_use]
pub fn protect(sddl: &str) -> String {
    if is_protected(sddl) {
        return sddl.to_owned();
    }
    let (flags, aces) = split(sddl);
    // Explicit deny ACEs must come first to keep the DACL in canonical order.
    let mut out = format!("D:{flags}{DENY_CHANGES}{OWNER_READS}");
    for ace in aces {
        out.push_str(ace);
    }
    out
}

/// Removes the protection ACEs added by [`protect`].
#[must_use]
pub fn unprotect(sddl: &str) -> String {
    let (flags, aces) = split(sddl);
    let mut out = format!("D:{flags}");
    for ace in aces.into_iter().filter(|ace| !is_ours(ace)) {
        out.push_str(ace);
    }
    out
}

#[must_use]
pub fn is_protected(sddl: &str) -> bool {
    split(sddl).1.iter().any(|ace| is_ours(ace))
}

/// Whether the DACL is protected from inheriting its parent's ACEs.
#[must_use]
pub fn blocks_inheritance(sddl: &str) -> bool {
    split(sddl).0.contains('P')
}

/// Our ACEs are the explicit (not inherited) ones for interactive users
/// that deny, and for OWNER RIGHTS. Windows may print the rights as
/// mnemonics rather than as the hex we wrote, so only the type, flags and
/// trustee are compared.
fn is_ours(ace: &str) -> bool {
    let fields: Vec<&str> = ace.trim_matches(['(', ')']).split(';').collect();
    let [kind, flags, _rights, _, _, trustee, ..] = fields[..] else {
        return false;
    };
    let explicit = !flags.to_ascii_uppercase().contains("ID");
    let trustee = trustee.to_ascii_uppercase();
    explicit && ((kind.eq_ignore_ascii_case("D") && trustee == "IU") || trustee == "OW")
}

/// Splits `D:<flags>(ace)(ace)...` into the flags and the ACEs, each with
/// its parentheses. Conditional ACEs may nest parentheses.
fn split(sddl: &str) -> (&str, Vec<&str>) {
    let body = sddl.strip_prefix("D:").unwrap_or(sddl);
    let first = body.find('(').unwrap_or(body.len());
    let (flags, rest) = body.split_at(first);
    let mut aces = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (i, c) in rest.char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    aces.push(&rest[start..=i]);
                }
            }
            _ => {}
        }
    }
    (flags, aces)
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER_DIR: &str = "D:AI(A;OICIID;FA;;;SY)(A;OICIID;FA;;;BA)\
        (A;OICIID;FA;;;S-1-5-21-1-2-3-1001)";

    #[test]
    fn protect_puts_the_deny_first_and_keeps_the_rest() {
        let protected = protect(USER_DIR);
        assert_eq!(
            protected,
            "D:AI(D;OICI;0xd0156;;;IU)(A;OICI;RC;;;OW)(A;OICIID;FA;;;SY)\
             (A;OICIID;FA;;;BA)(A;OICIID;FA;;;S-1-5-21-1-2-3-1001)"
        );
        assert!(is_protected(&protected));
        assert!(!is_protected(USER_DIR));
    }

    #[test]
    fn protect_is_idempotent() {
        let once = protect(USER_DIR);
        assert_eq!(protect(&once), once);
    }

    #[test]
    fn unprotect_restores_the_original() {
        assert_eq!(unprotect(&protect(USER_DIR)), USER_DIR);
        assert_eq!(unprotect(USER_DIR), USER_DIR);
    }

    #[test]
    fn unprotect_recognizes_rights_printed_as_mnemonics() {
        let read_back = "D:AI(D;OICI;DCLCRPDTCRSDWDWO;;;IU)(A;OICI;RC;;;OW)(A;OICIID;FA;;;SY)";
        assert_eq!(unprotect(read_back), "D:AI(A;OICIID;FA;;;SY)");
    }

    #[test]
    fn inherited_entries_are_never_removed() {
        let inherited = "D:AI(D;OICIID;FA;;;IU)(A;OICIID;FA;;;SY)";
        assert!(!is_protected(inherited));
        assert_eq!(unprotect(inherited), inherited);
    }

    #[test]
    fn reads_inheritance_flags() {
        assert!(blocks_inheritance("D:PAI(A;;FA;;;SY)"));
        assert!(!blocks_inheritance(USER_DIR));
    }

    #[test]
    fn handles_nested_parentheses() {
        let conditional = "D:(XA;;FX;;;S-1-1-0;(@User.Title == \"PM\"))(A;;FA;;;SY)";
        assert_eq!(split(conditional).1.len(), 2);
        assert_eq!(unprotect(&protect(conditional)), conditional);
    }
}
