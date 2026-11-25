use super::*;

#[derive(Debug, Clone)]
pub(super) struct PlatformDefinition {
    pub(super) id: &'static str,
    pub(super) name: &'static str,
    pub(super) category: &'static str,
    pub(super) category_label: &'static str,
    pub(super) content_formats: &'static str,
    pub(super) content_read: &'static str,
    pub(super) content_publish: &'static str,
    pub(super) analytics: &'static str,
    pub(super) login_url: &'static str,
    pub(super) auth_kind: AuthKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuthKind {
    BrowserCookie,
    ManualOnly,
    OAuthUnavailable,
}

pub(super) fn platform_definition(platform_id: &str) -> Option<PlatformDefinition> {
    let definitions = [
        PlatformDefinition {
            id: "juejin",
            name: "掘金",
            category: "tech",
            category_label: "技术社区",
            content_formats: "技术文章",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            login_url: "https://juejin.cn/",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "csdn",
            name: "CSDN",
            category: "tech",
            category_label: "技术社区",
            content_formats: "技术博客",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            login_url: "https://www.csdn.net/",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "zhihu",
            name: "知乎",
            category: "article",
            category_label: "深度图文",
            content_formats: "问答、文章",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            login_url: "https://www.zhihu.com/",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "wechat",
            name: "微信公众号",
            category: "article",
            category_label: "深度图文",
            content_formats: "图文、视频",
            content_read: "restricted",
            content_publish: "unsupported",
            analytics: "restricted",
            login_url: "https://mp.weixin.qq.com/",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "bilibili",
            name: "Bilibili",
            category: "video",
            category_label: "视频平台",
            content_formats: "视频、专栏",
            content_read: "supported",
            content_publish: "restricted",
            analytics: "supported",
            login_url: "https://www.bilibili.com/",
            auth_kind: AuthKind::BrowserCookie,
        },
        PlatformDefinition {
            id: "youtube",
            name: "YouTube",
            category: "video",
            category_label: "视频平台",
            content_formats: "视频、频道",
            content_read: "experimental",
            content_publish: "experimental",
            analytics: "experimental",
            login_url: "https://studio.youtube.com/",
            auth_kind: AuthKind::OAuthUnavailable,
        },
        PlatformDefinition {
            id: "douyin",
            name: "抖音",
            category: "video",
            category_label: "视频平台",
            content_formats: "短视频、图文",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            login_url: "https://creator.douyin.com/",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "kuaishou",
            name: "快手",
            category: "video",
            category_label: "视频平台",
            content_formats: "短视频",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            login_url: "https://cp.kuaishou.com/",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "xiaohongshu",
            name: "小红书",
            category: "social",
            category_label: "社交与种草",
            content_formats: "图文、视频",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            login_url: "https://creator.xiaohongshu.com/",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "weibo",
            name: "微博",
            category: "social",
            category_label: "社交与种草",
            content_formats: "社交动态",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            login_url: "https://weibo.com/",
            auth_kind: AuthKind::ManualOnly,
        },
    ];
    definitions
        .into_iter()
        .find(|definition| definition.id == platform_id)
}

fn browser_steps() -> Vec<String> {
    vec![
        "点击“打开登录页”，在当前系统浏览器完成平台登录；CreatorRelay 不读取浏览器主配置目录。".to_string(),
        "登录后从平台请求中复制完整 Cookie 请求头；只复制自己账号的会话值。".to_string(),
        "返回 CreatorRelay 粘贴 Cookie。后端会先调用平台身份接口校验，校验通过后才写入系统 Keychain。".to_string(),
    ]
}

fn manual_steps(platform_name: &str) -> Vec<String> {
    vec![
        format!("在系统浏览器打开 {platform_name} 并完成人工登录。"),
        "创建、检查与发布内容需要在平台官方页面内人工完成。".to_string(),
        "在 CreatorRelay 中仅保留账号别名与运营记录；人工登录不会被标记为已验证会话。".to_string(),
    ]
}

pub(super) fn auth_methods(definition: &PlatformDefinition) -> Vec<PlatformAuthMethod> {
    match definition.auth_kind {
        AuthKind::BrowserCookie => vec![
            PlatformAuthMethod {
                method: "browser".to_string(),
                available: true,
                label: "系统浏览器 + 会话校验".to_string(),
                login_url: Some(definition.login_url.to_string()),
                manual_steps: browser_steps(),
                reason: None,
            },
            PlatformAuthMethod {
                method: "embeddedWebView".to_string(),
                available: false,
                label: "内置 WebView".to_string(),
                login_url: None,
                manual_steps: Vec::new(),
                reason: Some("当前桌面构建尚未启用账号级独立 WebView 会话。".to_string()),
            },
        ],
        AuthKind::ManualOnly => vec![PlatformAuthMethod {
            method: "manual".to_string(),
            available: true,
            label: "系统浏览器人工流程".to_string(),
            login_url: Some(definition.login_url.to_string()),
            manual_steps: manual_steps(definition.name),
            reason: Some("该平台尚未完成官方身份接口验证，不会伪造 ACTIVE 会话。".to_string()),
        }],
        AuthKind::OAuthUnavailable => vec![
            PlatformAuthMethod {
                method: "oauth".to_string(),
                available: false,
                label: "官方 OAuth".to_string(),
                login_url: None,
                manual_steps: Vec::new(),
                reason: Some("尚未配置官方 OAuth 客户端与回调验证。".to_string()),
            },
            PlatformAuthMethod {
                method: "manual".to_string(),
                available: true,
                label: "系统浏览器人工流程".to_string(),
                login_url: Some(definition.login_url.to_string()),
                manual_steps: manual_steps(definition.name),
                reason: None,
            },
        ],
    }
}

pub(super) fn capabilities(definition: &PlatformDefinition) -> PlatformCapabilities {
    PlatformCapabilities {
        content_read: definition.content_read.to_string(),
        content_publish: definition.content_publish.to_string(),
        analytics: definition.analytics.to_string(),
    }
}

fn catalog_entry(definition: &PlatformDefinition) -> PlatformCatalogEntry {
    PlatformCatalogEntry {
        id: definition.id.to_string(),
        name: definition.name.to_string(),
        category: definition.category.to_string(),
        category_label: definition.category_label.to_string(),
        content_formats: definition.content_formats.to_string(),
        capabilities: capabilities(definition),
        auth_methods: auth_methods(definition),
    }
}

pub fn platform_catalog() -> Vec<PlatformCatalogEntry> {
    [
        "juejin",
        "csdn",
        "zhihu",
        "wechat",
        "bilibili",
        "youtube",
        "douyin",
        "kuaishou",
        "xiaohongshu",
        "weibo",
    ]
    .into_iter()
    .filter_map(platform_definition)
    .map(|definition| catalog_entry(&definition))
    .collect()
}

