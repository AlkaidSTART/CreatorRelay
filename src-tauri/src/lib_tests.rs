
use super::*;

fn test_db() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    database::migrate(&mut connection).unwrap();
    connection
}

#[test]
fn parses_frontmatter_title_and_tags() {
    let source =
        "---\ntitle: \"Draft title\"\ntags:\n  - Rust\n  - Writing\nstatus: draft\n---\n\n# Body";
    let (title, tags) = parse_frontmatter(source);
    assert_eq!(title, "Draft title");
    assert_eq!(tags, vec!["Rust", "Writing"]);
}

#[test]
fn extracts_body_without_frontmatter_for_metrics() {
    let source = "---\nid: doc\ntitle: test\n---\n\n# Hello\n\nworld";
    assert_eq!(body_for_word_count(source), "\n# Hello\n\nworld");
}

#[test]
fn search_matches_title_body_and_tags_case_insensitively() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("article.md");
    let content =
        "---\ntitle: Rust Notes\ntags:\n  - CreatorRelay\n---\n\nA private body phrase.\n";
    fs::write(&file, content).unwrap();
    let document = document_from_file("doc".to_string(), &file, temp.path()).unwrap();
    assert!(document_matches_query(&document, &file, "rust").unwrap());
    assert!(document_matches_query(&document, &file, "PRIVATE BODY").unwrap());
    assert!(document_matches_query(&document, &file, "creatorrelay").unwrap());
    assert!(!document_matches_query(&document, &file, "not present").unwrap());
    assert!(document_matches_query(&document, &file, "").unwrap());
}

#[test]
fn document_paths_cannot_escape_or_follow_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join("articles")).unwrap();
    assert!(resolve_document(&workspace, "../outside.md").is_err());
    assert!(resolve_document(&workspace, "/tmp/outside.md").is_err());
    #[cfg(unix)]
    {
        let outside = temp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(outside, workspace.join("articles/link")).unwrap();
        assert!(resolve_document(&workspace, "articles/link/escaped.md").is_err());
    }
}

#[test]
fn workspace_and_index_survive_database_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let database_path = temp.path().join("creatorrelay.sqlite3");
    let workspace = temp.path().join("workspace");
    setup_workspace(&workspace).unwrap();
    let file = workspace.join("articles/doc/article.md");
    let content = "---\ntitle: Restart proof\ntags: []\n---\n\n# Persisted\n";
    atomic_write(&file, content).unwrap();

    {
        let mut conn = Connection::open(&database_path).unwrap();
        database::migrate(&mut conn).unwrap();
        set_workspace(&conn, &workspace).unwrap();
        sync_index(&conn, &workspace, None).unwrap();
    }

    let mut reopened = Connection::open(&database_path).unwrap();
    database::migrate(&mut reopened).unwrap();
    assert_eq!(active_workspace(&reopened).unwrap().unwrap(), workspace);
    let count: i64 = reopened
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
    let title: String = reopened
        .query_row("SELECT title FROM documents", [], |row| row.get(0))
        .unwrap();
    assert_eq!(title, "Restart proof");
    assert_eq!(fs::read_to_string(&file).unwrap(), content);
    assert_eq!(
        database::current_version(&reopened).unwrap(),
        database::CURRENT_SCHEMA_VERSION
    );
}

#[test]
fn markdown_is_authoritative_and_index_is_rebuildable() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    setup_workspace(&workspace).unwrap();
    let path = workspace.join("articles/first/article.md");
    atomic_write(&path, "---\ntitle: First\ntags: []\n---\n\n# First\n").unwrap();
    let connection = test_db();
    sync_index(&connection, &workspace, None).unwrap();
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
    fs::write(&path, "---\ntitle: Updated\ntags: []\n---\n\n# Updated\n").unwrap();
    sync_index(&connection, &workspace, None).unwrap();
    let title: String = connection
        .query_row("SELECT title FROM documents", [], |row| row.get(0))
        .unwrap();
    assert_eq!(title, "Updated");
}

#[test]
fn sync_index_records_external_article_changes_in_git_history() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    setup_workspace(&workspace).unwrap();
    let history_root = temp.path().join("app-data/article-history");
    let path = workspace.join("articles/doc/article.md");
    let original = "# Original\n";
    atomic_write(&path, original).unwrap();
    let connection = test_db();
    set_workspace(&connection, &workspace).unwrap();
    sync_index(&connection, &workspace, Some(&history_root)).unwrap();
    let external = "# External\n";
    fs::write(&path, external).unwrap();
    sync_index(&connection, &workspace, Some(&history_root)).unwrap();
    let relative = "articles/doc/article.md";
    let repository = open_history_repository(&history_root, &workspace).unwrap();
    let revisions = list_document_revisions(&repository, relative).unwrap();
    assert_eq!(revisions.len(), 2);
    assert!(
        revisions
            .iter()
            .any(|revision| revision.preview == "# Original")
    );
    assert!(
        revisions
            .iter()
            .any(|revision| revision.preview == "# External")
    );
    assert_eq!(fs::read_to_string(path).unwrap(), external);
}

#[test]
fn workspace_assets_are_confined_to_article_assets_and_reject_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let article_dir = workspace.join("articles/doc");
    fs::create_dir_all(article_dir.join("assets")).unwrap();
    fs::write(article_dir.join("article.md"), "# Article").unwrap();
    fs::write(article_dir.join("assets/image.png"), b"image").unwrap();
    let normal =
        resolve_workspace_asset(&workspace, "articles/doc/article.md", "assets/image.png").unwrap();
    assert_eq!(normal, article_dir.join("assets/image.png"));
    assert!(
        resolve_workspace_asset(&workspace, "articles/doc/article.md", "../../outside.png",)
            .is_err()
    );
    #[cfg(unix)]
    {
        let outside = temp.path().join("outside.png");
        fs::write(&outside, b"outside").unwrap();
        std::os::unix::fs::symlink(outside, article_dir.join("assets/linked.png")).unwrap();
        assert!(
            resolve_workspace_asset(&workspace, "articles/doc/article.md", "assets/linked.png",)
                .is_err()
        );
    }
}

#[test]
fn legacy_relative_paths_stay_bound_to_the_original_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("original");
    let next = temp.path().join("next");
    setup_workspace(&original).unwrap();
    setup_workspace(&next).unwrap();
    let original_file = original.join("articles/doc/article.md");
    atomic_write(&original_file, "# Original\n").unwrap();
    let connection = test_db();
    set_workspace(&connection, &original).unwrap();
    connection
        .execute(
            "INSERT INTO documents VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                "legacy",
                "articles/doc/article.md",
                "Original",
                Utc::now().to_rfc3339(),
                digest("# Original\n")
            ],
        )
        .unwrap();
    normalize_legacy_paths(&connection, &next).unwrap();
    let migrated: String = connection
        .query_row("SELECT path FROM documents WHERE id='legacy'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        PathBuf::from(migrated),
        original_file.canonicalize().unwrap()
    );
}

#[test]
fn same_relative_path_in_distinct_workspaces_has_distinct_identity() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    setup_workspace(&first).unwrap();
    setup_workspace(&second).unwrap();
    let first_doc = first.join("articles/shared/article.md");
    let second_doc = second.join("articles/shared/article.md");
    atomic_write(&first_doc, "# First\n").unwrap();
    atomic_write(&second_doc, "# Second\n").unwrap();
    let connection = test_db();
    sync_index(&connection, &first, None).unwrap();
    sync_index(&connection, &second, None).unwrap();
    let rows: i64 = connection
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 2);
}

#[test]
fn git_history_tracks_manual_article_versions_and_path_scoped_diffs() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    setup_workspace(&workspace).unwrap();
    let history_root = temp.path().join("app-data/article-history");
    let article_path = "articles/doc-a/article.md";
    let first = "---\ntitle: Draft\ncustom: preserved\n---\n\n# Draft\n";
    let second = first.replace("# Draft", "# Updated");
    let other_path = "articles/doc-b/article.md";
    let other = "# Other article\n";
    let repo = open_history_repository(&history_root, &workspace).unwrap();

    commit_document_content(&repo, article_path, first, "创建文章", None).unwrap();
    commit_document_content(&repo, other_path, other, "创建另一篇", None).unwrap();
    commit_document_content(&repo, article_path, &second, "手动保存版本", None).unwrap();
    commit_document_content(&repo, article_path, &second, "重复快照", None).unwrap();

    let revisions = list_document_revisions(&repo, article_path).unwrap();
    assert_eq!(revisions.len(), 2);
    assert!(revisions[0].diff.contains("# Updated"));
    assert!(!revisions[0].diff.contains("Other article"));
    assert!(revisions[1].diff.contains("custom: preserved"));
}

#[test]
fn git_history_migrates_legacy_snapshots_without_changing_markdown_files() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    setup_workspace(&workspace).unwrap();
    let file = workspace.join("articles/doc/article.md");
    let current = "---\ntitle: Current\ncustom: keep\n---\n\n# Current\n";
    atomic_write(&file, current).unwrap();
    let before = digest(&fs::read_to_string(&file).unwrap());
    let connection = test_db();
    let created_at = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO documents VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                "doc",
                file.to_string_lossy(),
                "Current",
                created_at,
                digest(current)
            ],
        )
        .unwrap();
    let old = "---\ntitle: Old\ncustom: keep\n---\n\n# Old\n";
    connection.execute(
            "INSERT INTO revisions(document_id, content, content_hash, label, created_at) VALUES(?1, ?2, ?3, ?4, ?5)",
            params!["doc", old, digest(old), "旧版本", created_at],
        ).unwrap();
    let history_root = temp.path().join("history");
    let repo = open_history_repository(&history_root, &workspace).unwrap();
    ensure_document_history(
        &repo,
        &connection,
        "doc",
        "articles/doc/article.md",
        current,
    )
    .unwrap();

    let revisions = list_document_revisions(&repo, "articles/doc/article.md").unwrap();
    assert_eq!(revisions.len(), 2);
    assert!(
        revisions
            .iter()
            .any(|revision| revision.preview.contains("# Old"))
    );
    assert_eq!(digest(&fs::read_to_string(file).unwrap()), before);
}

#[test]
fn restoring_a_git_version_creates_a_new_recoverable_tip() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    setup_workspace(&workspace).unwrap();
    let root = temp.path().join("history");
    let path = "articles/doc/article.md";
    let first = "# First\n";
    let second = "# Second\n";
    let repo = open_history_repository(&root, &workspace).unwrap();
    commit_document_content(&repo, path, first, "创建", None).unwrap();
    let first_id = list_document_revisions(&repo, path).unwrap()[0].id.clone();
    commit_document_content(&repo, path, second, "更新", None).unwrap();
    let selected = repo
        .find_commit(git2::Oid::from_str(&first_id).unwrap())
        .unwrap();
    let selected_tree = selected.tree().unwrap();
    let selected_blob = repo
        .find_blob(tree_blob_id(&selected_tree, path).unwrap())
        .unwrap();
    let restored = std::str::from_utf8(selected_blob.content()).unwrap();
    commit_document_content(&repo, path, second, "恢复前保留当前版本", None).unwrap();
    commit_document_content(&repo, path, restored, "恢复选定版本", None).unwrap();

    let revisions = list_document_revisions(&repo, path).unwrap();
    assert_eq!(revisions.len(), 3);
    assert_eq!(revisions[0].preview, "# First");
    assert!(
        revisions
            .iter()
            .any(|revision| revision.preview == "# Second")
    );
}

#[test]
fn git_tree_preserves_workspace_files_but_commits_only_article_markdown() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    setup_workspace(&workspace).unwrap();
    fs::write(workspace.join("assets/image.png"), b"asset bytes").unwrap();
    fs::write(workspace.join("notes.txt"), "outside article scope").unwrap();
    let repo = open_history_repository(&temp.path().join("history"), &workspace).unwrap();
    commit_document_content(
        &repo,
        "articles/doc/article.md",
        "# Versioned article\n",
        "create",
        None,
    )
    .unwrap();
    let head = current_head(&repo).unwrap().unwrap();
    let tree = head.tree().unwrap();
    assert_eq!(tree.len(), 1);
    assert!(tree.get_name("articles").is_some());
    assert!(tree.get_name("notes.txt").is_none());
    assert!(workspace.join("assets/image.png").exists());
    assert!(!workspace.join(".git").exists());
}

#[test]
fn index_rebuild_only_reindexes_markdown_and_does_not_change_files() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    setup_workspace(&workspace).unwrap();
    let article = workspace.join("articles/doc/article.md");
    let content = "---\ntitle: Rebuild\ncustom: value\n---\n\n# Keep me\n";
    atomic_write(&article, content).unwrap();
    let before = digest(&fs::read_to_string(&article).unwrap());
    let db = test_db();
    sync_index(&db, &workspace, None).unwrap();
    db.execute("DELETE FROM documents", []).unwrap();
    sync_index(&db, &workspace, None).unwrap();
    let title: String = db
        .query_row("SELECT title FROM documents", [], |row| row.get(0))
        .unwrap();
    assert_eq!(title, "Rebuild");
    assert_eq!(digest(&fs::read_to_string(&article).unwrap()), before);
    fs::remove_file(&article).unwrap();
    sync_index(&db, &workspace, None).unwrap();
    let remaining: i64 = db
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .unwrap();
    assert_eq!(remaining, 0);
}

#[test]
fn referenced_assets_are_listed_and_missing_assets_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let article_dir = workspace.join("articles/doc");
    fs::create_dir_all(article_dir.join("assets")).unwrap();
    fs::write(article_dir.join("article.md"), "# Article").unwrap();
    fs::write(article_dir.join("assets/image.png"), b"image").unwrap();
    let markdown = "# Article\n\n![alt](assets/image.png)\n";
    let assets = list_document_assets(&workspace, "articles/doc/article.md", markdown).unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].mime_type, "image/png");
    let missing = list_document_assets(
        &workspace,
        "articles/doc/article.md",
        "![missing](assets/missing.png)",
    )
    .unwrap();
    assert_eq!(missing.len(), 1);
    assert!(!missing[0].available);
    assert!(missing[0].warning.is_some());
}

#[test]
fn article_asset_references_are_checked_before_saving() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    setup_workspace(&workspace).unwrap();
    let document_dir = workspace.join("articles/doc");
    fs::create_dir_all(document_dir.join("assets")).unwrap();
    fs::write(document_dir.join("assets/image.png"), b"image").unwrap();
    assert!(
        validate_asset_references(
            &workspace,
            "articles/doc/article.md",
            "![image](assets/image.png)",
        )
        .is_ok()
    );
    assert!(
        validate_asset_references(
            &workspace,
            "articles/doc/article.md",
            "![missing](assets/missing.png)",
        )
        .is_err()
    );
    assert!(
        validate_asset_references(
            &workspace,
            "articles/doc/article.md",
            "![escape](../../outside.png)",
        )
        .is_err()
    );
    assert!(
        validate_asset_references(
            &workspace,
            "articles/doc/article.md",
            "[external](https://example.com/article)",
        )
        .is_ok()
    );
    assert!(
        validate_asset_references(
            &workspace,
            "articles/doc/article.md",
            "![remote](https://example.com/cover.png)",
        )
        .is_ok()
    );
    assert!(
        validate_asset_references(
            &workspace,
            "articles/doc/article.md",
            "![other](other/image.png)",
        )
        .is_err()
    );
}

#[test]
fn save_and_restore_base_hash_reject_external_changes() {
    let original = "# Original\n";
    let external = "# External\n";
    assert!(ensure_base_hash(original, Some(&digest(original))).is_ok());
    let error = ensure_base_hash(external, Some(&digest(original))).unwrap_err();
    assert!(error.contains("其他位置修改"));
    assert!(ensure_base_hash(external, None).is_ok());
}
