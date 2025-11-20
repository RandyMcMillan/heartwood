use std::{
    fs::{metadata, set_permissions, write},
    os::unix::fs::PermissionsExt,
    path::Path,
};

use crate::adapter::Adapter;
use crate::ci_event::{CiEvent, CiEventV1};
use crate::msg::{Request, RequestBuilder};
use radicle::crypto::ssh::Keystore;
use radicle::crypto::test::signer::MockSigner;
use radicle::crypto::Signer;
use radicle::git::RefString;
use radicle::profile::{Config, Home};
use radicle::storage::ReadRepository;
use radicle::test::setup::Node;
use radicle::Profile;

use tempfile::tempdir;

pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct MockNode {
    alias: String,
    node: Node,
}

impl MockNode {
    pub fn new() -> TestResult<Self> {
        let tmp = tempdir()?;
        let signer = MockSigner::default();
        let alias = "broker_test_alias".to_string();
        let node = Node::new(tmp, signer, &alias);
        Ok(Self { alias, node })
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn node(&self) -> &Node {
        &self.node
    }

    pub fn profile(&self) -> Result<Profile, Box<dyn std::error::Error>> {
        let node = self.node();
        Ok(Profile {
            home: Home::new(node.root.clone())?,
            storage: node.storage.clone(),
            keystore: Keystore::new(&node.root),
            public_key: node.signer.public_key().to_owned(),
            config: Config::new(self.alias().parse()?),
        })
    }
}

pub fn trigger_request() -> TestResult<Request> {
    let mock_node = MockNode::new()?;
    let profile = mock_node.profile()?;
    let node = mock_node.node();

    let project = node.project();
    let repo_head = project.repo.head()?.1;
    let cmt =
        radicle::test::fixtures::commit("my test commit", &[repo_head.into()], &project.backend);

    let ci_event = CiEvent::V1(CiEventV1::BranchCreated {
        from_node: *profile.id(),
        repo: project.id,
        branch: RefString::try_from(
            "refs/namespaces/$nid/refs/heads/master".replace("$nid", &profile.id().to_string()),
        )?,
        tip: cmt,
    });

    Ok(RequestBuilder::default()
        .profile(&profile)
        .ci_event(&ci_event)
        .build_trigger_from_ci_event()?)
}

pub fn mock_adapter(filename: &Path, script: &str) -> TestResult<Adapter> {
    const EXECUTABLE: u32 = 0o100;

    write(filename, script)?;
    let meta = metadata(filename)?;
    let mut perms = meta.permissions();
    perms.set_mode(perms.mode() | EXECUTABLE);
    set_permissions(filename, perms)?;

    Ok(Adapter::new(filename))
}
