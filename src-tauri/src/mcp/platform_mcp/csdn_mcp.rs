use super::*;

pub(super) fn profile() -> PlatformMcpProfile {
    PlatformMcpProfile {
        server_name: "csdn_mcp".to_string(),
        platform_id: "csdn".to_string(),
        platform_name: "CSDN".to_string(),
        description: "CSDN 工具面：创作中心文章编辑器、文章 ID、标题/正文/标签/封面字段与阅读互动指标。当前无已核实的官方发布 API，使用人工网页交接。".to_string(),
        transport: "core_bridge".to_string(),
        auth_mode: "browser_session_manual".to_string(),
        admission_status: "evaluation_only".to_string(),
        auto_publish_allowed: false,
        publish_dispatch: "manual_only".to_string(),
        native_work_id_field: "articleId".to_string(),
        interfaces: vec![
            interface(
                "article_editor",
                "write",
                "manual_web",
                None,
                Some("https://mp.csdn.net/mp_blog/creation/editor"),
                "人工登录会话",
                "manual_web",
                "https://mp.csdn.net/",
                "创作中心字段：标题、Markdown 正文、标签、封面、分类/专栏。",
            ),
            interface(
                "content_manage",
                "read",
                "manual_web",
                None,
                Some("https://mp.csdn.net/mp_blog/manage/article"),
                "人工登录会话",
                "manual_web",
                "https://mp.csdn.net/",
                "文章以 articleId 标识；列表可查看阅读、点赞、评论、收藏。",
            ),
        ],
        tools: vec![
            account_tool("csdn", "读取 CSDN 账号", "读取 Core 中 CSDN 账号的脱敏状态与人工发布准备情况。"),
            work_list_tool(
                "csdn",
                "articleId",
                "CSDN 文章",
                json!({
                    "column": optional_string_property(80),
                    "tag": optional_string_property(64),
                }),
                "按 articleId、专栏、标签、发布时间和关键词查询已同步的 CSDN 文章。",
            ),
            analytics_tool(
                "csdn",
                "articleId",
                "CSDN 文章",
                vec!["readCount", "diggCount", "commentCount", "collectCount"],
                json!({
                    "column": optional_string_property(80),
                }),
            ),
            publish_validate_tool("csdn", "CSDN", "articleId", "CSDN 文章"),
        ],
        field_mappings: vec![
            mapping("title", "title", "write", true, "文章标题。"),
            mapping("summary", "description", "write", false, "文章摘要/描述。"),
            mapping("body", "content", "write", true, "Markdown/富文本正文。"),
            mapping("tags", "tags", "write", false, "文章标签。"),
            mapping("cover", "cover", "write", false, "封面图片。"),
            mapping("externalId", "articleId", "read", true, "文章回读主键。"),
            mapping("metrics.view", "readCount", "read", true, "阅读数。"),
            mapping("metrics.like", "diggCount", "read", true, "点赞数。"),
            mapping("metrics.comment", "commentCount", "read", true, "评论数。"),
            mapping("metrics.collect", "collectCount", "read", true, "收藏数。"),
        ],
        content_rules: vec![
            content_rule("article", vec![], 0, None, None, None, None, false, false, "技术文章必须有正文；封面可选。"),
            content_rule("imageArticle", vec!["image"], 1, None, None, None, None, false, false, "图文文章按 Markdown + 图片素材交接。"),
        ],
        handoff: handoff(
            "https://mp.csdn.net/mp_blog/creation/editor",
            "https://mp.csdn.net/mp_blog/manage/article",
            Some("https://mp.csdn.net/mp_blog/manage/article"),
            vec![
                "校验标题、摘要、Markdown 正文、标签和封面。",
                "在 CSDN 创作中心人工粘贴正文并发布。",
                "发布后复制 articleId 或链接回填。",
            ],
            "CSDN 当前没有已核实的官方文章发布 API；csdn_mcp 不模拟网页内部接口。",
        ),
    }
}
