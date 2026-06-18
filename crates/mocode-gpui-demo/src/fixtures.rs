use crate::component::GpuiEditorDocument;
use mocode_api::TextPosition;

pub(crate) const SAMPLE_TITLE: &str = "examples/configs/dialer-proxy.yaml";
pub(crate) const SAMPLE_TEXT: &str = include_str!("../../../examples/configs/dialer-proxy.yaml");
const INSPECT_POSITION: TextPosition = TextPosition::new(10, 17);

pub(crate) const DEFAULT_FIXTURE_ID: &str = "dialer-proxy";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DemoFixture {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) title: &'static str,
    pub(crate) text: &'static str,
    pub(crate) inspect_position: TextPosition,
}

const DEMO_FIXTURES: &[DemoFixture] = &[
    DemoFixture {
        id: "dialer-proxy",
        label: "Dialer",
        title: SAMPLE_TITLE,
        text: SAMPLE_TEXT,
        inspect_position: INSPECT_POSITION,
    },
    DemoFixture {
        id: "minimal",
        label: "Minimal",
        title: "examples/configs/minimal.yaml",
        text: include_str!("../../../examples/configs/minimal.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
    DemoFixture {
        id: "dns",
        label: "DNS",
        title: "examples/configs/dns.yaml",
        text: include_str!("../../../examples/configs/dns.yaml"),
        inspect_position: TextPosition::new(2, 16),
    },
    DemoFixture {
        id: "tun",
        label: "TUN",
        title: "examples/configs/tun.yaml",
        text: include_str!("../../../examples/configs/tun.yaml"),
        inspect_position: TextPosition::new(2, 4),
    },
    DemoFixture {
        id: "proxy-groups",
        label: "Groups",
        title: "examples/configs/proxy-groups.yaml",
        text: include_str!("../../../examples/configs/proxy-groups.yaml"),
        inspect_position: TextPosition::new(7, 8),
    },
    DemoFixture {
        id: "providers",
        label: "Providers",
        title: "examples/configs/providers.yaml",
        text: include_str!("../../../examples/configs/providers.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
    DemoFixture {
        id: "invalid-yaml",
        label: "Bad YAML",
        title: "examples/configs/invalid-yaml.yaml",
        text: include_str!("../../../examples/configs/invalid-yaml.yaml"),
        inspect_position: TextPosition::new(2, 0),
    },
    DemoFixture {
        id: "invalid-reference",
        label: "Bad Ref",
        title: "examples/configs/invalid-reference.yaml",
        text: include_str!("../../../examples/configs/invalid-reference.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
    DemoFixture {
        id: "dialer-cycle",
        label: "Cycle",
        title: "tests/fixtures/dialer-cycle.yaml",
        text: include_str!("../../../tests/fixtures/dialer-cycle.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
    DemoFixture {
        id: "large",
        label: "Large",
        title: "examples/configs/large.yaml",
        text: include_str!("../../../examples/configs/large.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
    DemoFixture {
        id: "large-20000",
        label: "20k",
        title: "examples/configs/large-20000.yaml",
        text: include_str!("../../../examples/configs/large-20000.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
];

pub(crate) fn all_fixtures() -> &'static [DemoFixture] {
    DEMO_FIXTURES
}

pub(crate) fn default_fixture() -> &'static DemoFixture {
    fixture_by_id(DEFAULT_FIXTURE_ID).expect("default fixture must exist")
}

pub(crate) fn fixture_by_id(id: &str) -> Option<&'static DemoFixture> {
    DEMO_FIXTURES.iter().find(|fixture| fixture.id == id)
}

pub(crate) fn document_from_fixture(fixture: &DemoFixture) -> GpuiEditorDocument {
    GpuiEditorDocument::from_text(fixture.title, fixture.text, fixture.inspect_position)
}

pub(crate) fn default_document() -> GpuiEditorDocument {
    document_from_fixture(default_fixture())
}

pub(crate) fn document_by_fixture_id(id: &str) -> Option<GpuiEditorDocument> {
    fixture_by_id(id).map(document_from_fixture)
}
