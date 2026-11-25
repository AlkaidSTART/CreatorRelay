import type { CSSProperties } from "react";

import bilibiliLogo from "../assets/platforms/bilibili.svg";
import csdnLogo from "../assets/platforms/csdn.svg";
import douyinLogo from "../assets/platforms/douyin.svg";
import juejinLogo from "../assets/platforms/juejin.svg";
import kuaishouLogo from "../assets/platforms/kuaishou.svg";
import wechatLogo from "../assets/platforms/wechat.svg";
import weiboLogo from "../assets/platforms/weibo.svg";
import xiaohongshuLogo from "../assets/platforms/xiaohongshu.svg";
import youtubeLogo from "../assets/platforms/youtube.svg";
import zhihuLogo from "../assets/platforms/zhihu.svg";

const PLATFORM_LOGOS: Record<string, string> = {
  bilibili: bilibiliLogo,
  csdn: csdnLogo,
  douyin: douyinLogo,
  juejin: juejinLogo,
  kuaishou: kuaishouLogo,
  wechat: wechatLogo,
  weibo: weiboLogo,
  xiaohongshu: xiaohongshuLogo,
  youtube: youtubeLogo,
  zhihu: zhihuLogo,
};

const PLATFORM_ALIASES: Record<string, string> = {
  b站: "bilibili",
  哔哩哔哩: "bilibili",
  bilibili: "bilibili",
  csdn: "csdn",
  抖音: "douyin",
  douyin: "douyin",
  掘金: "juejin",
  juejin: "juejin",
  快手: "kuaishou",
  kuaishou: "kuaishou",
  微信: "wechat",
  微信公众号: "wechat",
  wechat: "wechat",
  微博: "weibo",
  weibo: "weibo",
  sinaweibo: "weibo",
  小红书: "xiaohongshu",
  xiaohongshu: "xiaohongshu",
  youtube: "youtube",
  知乎: "zhihu",
  zhihu: "zhihu",
};

export function resolvePlatformLogoId(platformId?: string, platformName?: string) {
  const id = platformId?.trim().toLocaleLowerCase();
  if (id && PLATFORM_LOGOS[id]) return id;

  const name = platformName?.trim().toLocaleLowerCase();
  return name ? PLATFORM_ALIASES[name] : undefined;
}

export interface PlatformLogoProps {
  platformId?: string;
  platformName?: string;
  size?: number | string;
  className?: string;
  style?: CSSProperties;
}

export function PlatformLogo({
  platformId,
  platformName,
  size = 16,
  className = "",
  style,
}: PlatformLogoProps) {
  const logoId = resolvePlatformLogoId(platformId, platformName);
  const src = logoId ? PLATFORM_LOGOS[logoId] : undefined;

  if (!src) {
    return (
      <span
        className={`inline-grid shrink-0 place-items-center font-semibold ${className}`.trim()}
        style={{ width: size, height: size, ...style }}
        aria-hidden="true"
      >
        {(platformName ?? platformId ?? "P").slice(0, 1).toUpperCase()}
      </span>
    );
  }

  return (
    <img
      src={src}
      width={size}
      height={size}
      className={`shrink-0 object-contain ${className}`.trim()}
      style={{ width: size, height: size, ...style }}
      alt=""
      aria-hidden="true"
      draggable={false}
      data-platform-logo={logoId}
    />
  );
}
