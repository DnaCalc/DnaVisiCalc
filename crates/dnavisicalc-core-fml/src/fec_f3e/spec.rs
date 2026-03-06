#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecClause {
    pub id: &'static str,
    pub summary: &'static str,
}

pub const FEC_F3E_INTERFACE_VERSION: &str = "fec-f3e-draft/v0";

pub const FEC_F3E_CLAUSES: &[SpecClause] = &[
    SpecClause {
        id: "FEC-F3E-OWN-001",
        summary: "F3E owns value/type semantics; FEC must not reinterpret them.",
    },
    SpecClause {
        id: "FEC-F3E-CALL-001",
        summary: "F3E compile/declare/evaluate calls are explicit and host-routed.",
    },
    SpecClause {
        id: "FEC-F3E-CAP-001",
        summary: "FEC exposes capability views scoped to declared requirements.",
    },
    SpecClause {
        id: "FEC-F3E-DEP-001",
        summary: "Dependency declaration and registration are tokenized and host-managed.",
    },
    SpecClause {
        id: "FEC-F3E-PUB-001",
        summary: "Evaluation publication is a FEC concern after F3E evaluation.",
    },
];

pub fn clause_by_id(id: &str) -> Option<SpecClause> {
    FEC_F3E_CLAUSES
        .iter()
        .copied()
        .find(|clause| clause.id == id)
}
