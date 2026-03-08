#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecClause {
    pub id: &'static str,
    pub summary: &'static str,
}

pub const FEC_F3E_INTERFACE_VERSION: &str = "fec-f3e-redesign/b1";

pub const FEC_F3E_CLAUSES: &[SpecClause] = &[
    SpecClause {
        id: "FEC-F3E-TXN-001",
        summary: "F3E exposes prepare/execute transactional semantic calls.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-002",
        summary: "FEC coordinates open-session/capability/commit atomically.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-003",
        summary: "Commit status is explicit: applied/token/capability/snapshot outcomes.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-004",
        summary: "Observed dependency deltas are first-class commit metadata.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-005",
        summary: "Spill shape transitions are first-class commit metadata.",
    },
];

pub fn clause_by_id(id: &str) -> Option<SpecClause> {
    FEC_F3E_CLAUSES
        .iter()
        .copied()
        .find(|clause| clause.id == id)
}
