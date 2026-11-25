use super::schema::{cover_ref_schema, media_ref_schema, object_schema, string_array_schema, tool};
use super::*;

pub fn tool_catalog() -> Vec<McpToolDefinition> {
    let mut tools = vec![
        tool(
            "account_get_info",
            "读取账号信息",
            "通过 Core 账号服务读取脱敏的账号状态与能力，不接受凭据引用。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "accountId": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "platformId": { "type": "string", "minLength": 1, "maxLength": 64 }
                }),
                vec![],
            ),
        ),
        tool(
            "platform_capabilities_get",
            "读取平台与账号能力",
            "返回平台准入、官方来源、逐形态读取/发布状态和账号级降级原因；未验证能力不会显示为 supported。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "platformId": { "type": "string", "minLength": 1, "maxLength": 64 },
                    "accountId": { "type": "string", "minLength": 1, "maxLength": 128 }
                }),
                vec![],
            ),
        ),
        tool(
            "creator_browser_status",
            "查看专用浏览器状态",
            "查看 CreatorRelay 专用 Chrome 是否运行及当前页面信息。不会连接用户日常浏览器配置。",
            ToolPermission::Read,
            object_schema(json!({}), vec![]),
        ),
        tool(
            "creator_browser_open",
            "在专用浏览器打开创作者中心",
            "仅在隔离的 CreatorRelay Chrome 配置中打开 SQLite 已配置的创作者中心 HTTPS 链接。页面会显示给用户；此工具不会提交表单或发布内容。",
            ToolPermission::ExternalAction,
            object_schema(
                json!({ "platformId": { "type": "string", "minLength": 1, "maxLength": 64 } }),
                vec!["platformId"],
            ),
        ),
        tool(
            "creator_browser_read_page",
            "读取当前创作者中心页面",
            "仅返回 MCP 专用浏览器中、通过 creator_browser_open 打开的已配置平台页面可见文本。不会执行页面脚本、读取 Cookie/隐藏网络响应或触发页面动作。",
            ToolPermission::Read,
            object_schema(
                json!({ "platformId": { "type": "string", "minLength": 1, "maxLength": 64 } }),
                vec!["platformId"],
            ),
        ),
        tool(
            "creator_centers_list",
            "列出创作者中心链接",
            "读取用户配置的创作者中心网址。该工具只返回链接配置，不会登录平台、读取平台实时作品或执行发布；请用 platform_capabilities_get 查看已验证能力。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "platformId": { "type": "string", "minLength": 1, "maxLength": 64 }
                }),
                vec![],
            ),
        ),
        tool(
            "content_list",
            "查询本地内容",
            "按关键词、形态和平台查询本地内容包，不返回素材原始路径或凭据。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "query": { "type": "string", "maxLength": 200 },
                    "contentKind": { "type": "string", "enum": ["video", "imageText", "imageArticle", "article"] },
                    "platformId": { "type": "string", "maxLength": 64 },
                    "accountId": { "type": "string", "maxLength": 128 },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                }),
                vec![],
            ),
        ),
        tool(
            "content_get_detail",
            "读取内容详情",
            "通过 Core 内容服务读取内容包详情；正文是否返回由本次模型上下文授权范围决定。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "contentId": { "type": "string", "minLength": 1, "maxLength": 128 }
                }),
                vec!["contentId"],
            ),
        ),
        tool(
            "external_content_list",
            "查询已发布作品",
            "查询已同步的外部作品及原始指标口径，不把不同形态的指标合并为总数。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "keyword": { "type": "string", "maxLength": 200 },
                    "platformId": { "type": "string", "maxLength": 64 },
                    "accountId": { "type": "string", "maxLength": 128 },
                    "contentKind": { "type": "string", "enum": ["video", "imageText", "imageArticle", "article", "unknown"] },
                    "publishedFrom": { "type": "string", "maxLength": 40 },
                    "publishedTo": { "type": "string", "maxLength": 40 }
                }),
                vec![],
            ),
        ),
        tool(
            "analytics_get_trend",
            "读取数据趋势",
            "按平台和内容形态读取可追溯指标趋势，保留平台原始指标口径。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "range": { "type": "string", "enum": ["7d", "30d", "90d"] },
                    "granularity": { "type": "string", "enum": ["day", "week", "month"] },
                    "platformId": { "type": "string", "maxLength": 64 },
                    "accountId": { "type": "string", "maxLength": 128 },
                    "contentKind": { "type": "string", "enum": ["video", "imageText", "imageArticle", "article"] },
                    "workId": { "type": "string", "maxLength": 128 }
                }),
                vec!["range", "granularity"],
            ),
        ),
        tool(
            "studio_create_content",
            "创建内容草稿",
            "通过受控 Core 资产服务创建视频、图文、图文文章或纯文章草稿；媒体只能引用已有资产 ID。",
            ToolPermission::Write,
            object_schema(
                json!({
                    "contentKind": { "type": "string", "enum": ["video", "imageText", "imageArticle", "article"] },
                    "title": { "type": "string", "minLength": 1, "maxLength": 200 },
                    "summary": { "type": "string", "maxLength": 2000 },
                    "tags": { "type": "array", "maxItems": 20, "items": { "type": "string", "minLength": 1, "maxLength": 40 } },
                    "body": { "type": "string", "maxLength": 400000 },
                    "media": { "type": "array", "maxItems": 30, "items": media_ref_schema() },
                    "cover": cover_ref_schema()
                }),
                vec!["contentKind", "title"],
            ),
        ),
        tool(
            "studio_update_content",
            "更新内容草稿",
            "更新本地内容草稿，保持原内容形态；媒体引用必须来自 Core 内容包资产。",
            ToolPermission::Write,
            object_schema(
                json!({
                    "contentId": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "title": { "type": "string", "minLength": 1, "maxLength": 200 },
                    "summary": { "type": "string", "maxLength": 2000 },
                    "tags": { "type": "array", "maxItems": 20, "items": { "type": "string", "minLength": 1, "maxLength": 40 } },
                    "body": { "type": "string", "maxLength": 400000 },
                    "media": { "type": "array", "maxItems": 30, "items": media_ref_schema() },
                    "cover": cover_ref_schema(),
                    "baseHash": { "type": "string", "minLength": 1, "maxLength": 128 }
                }),
                vec!["contentId"],
            ),
        ),
        tool(
            "publishing_create_task",
            "创建发布任务",
            "为明确的平台账号创建独立发布任务；不自动执行外部发布。",
            ToolPermission::Write,
            object_schema(
                json!({
                    "sourceType": { "type": "string", "enum": ["content", "platformDraft"] },
                    "sourceId": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "targets": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 12,
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "platformId": { "type": "string", "minLength": 1, "maxLength": 64 },
                                "platformName": { "type": "string", "minLength": 1, "maxLength": 80 },
                                "accountId": { "type": "string", "minLength": 1, "maxLength": 128 },
                                "accountAlias": { "type": "string", "minLength": 1, "maxLength": 80 },
                                "adapterId": { "type": "string", "maxLength": 128 }
                            },
                            "required": ["platformId", "platformName", "accountId", "accountAlias"]
                        }
                    },
                    "scheduledAt": { "type": "string", "maxLength": 40 }
                }),
                vec!["sourceType", "sourceId", "targets"],
            ),
        ),
        tool(
            "publishing_publish_task",
            "发布已确认内容",
            "External Action：按任务逐个发布到平台。每次执行必须展示全部目标与最终内容并重新确认。",
            ToolPermission::ExternalAction,
            object_schema(
                json!({
                    "taskIds": string_array_schema(12)
                }),
                vec!["taskIds"],
            ),
        ),
        tool(
            "content_delete_draft",
            "删除本地草稿",
            "删除指定内容包。该操作需要确认，且只接受内容 ID，不接受任意文件路径。",
            ToolPermission::Write,
            object_schema(
                json!({
                    "contentId": { "type": "string", "minLength": 1, "maxLength": 128 }
                }),
                vec!["contentId"],
            ),
        ),
        tool(
            "mcp_test_echo",
            "MCP 回显测试",
            "验证本地 MCP Server 生命周期、工具发现和协议调用。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "message": { "type": "string", "minLength": 1, "maxLength": 500 }
                }),
                vec!["message"],
            ),
        ),
        tool(
            "mcp_test_hang",
            "MCP 超时取消测试",
            "模拟长时间运行的本地工具，用于验证超时、取消和重复执行保护。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "milliseconds": { "type": "integer", "minimum": 100, "maximum": 120000 }
                }),
                vec!["milliseconds"],
            ),
        ),
    ];
    tools.extend(super::platform_mcp::platform_tool_definitions());
    tools
}

pub fn server_tool_descriptors() -> Vec<Value> {
    tool_catalog()
        .into_iter()
        .map(|item| {
            json!({
                "name": item.name,
                "title": item.title,
                "description": item.description,
                "inputSchema": item.input_schema
            })
        })
        .collect()
}
