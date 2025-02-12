export type SiteConfig = typeof siteConfig;

export const siteConfig = {
  name: "Nomos",
  description: "Multisig for cosmosverse",
  navItems: [
    {
      label: "Home",
      href: "/",
    },
    {
      label: "Utils",
      href: "/pages/utils",
    },
    {
      label: "Transfer",
      href: "/pages/transfer",
    },
    {
      label: "Transfer CW20",
      href: "/pages/transfer_cw20",
    }
  ],
  navMenuItems: [
    {
      label: "Home",
      href: "/",
    },
    {
      label: "Utils",
      href: "/pages/utils",
    },
    {
      label: "Transfer",
      href: "/pages/transfer",
    },
    {
      label: "Transfer CW20",
      href: "/pages/transfer_cw20",
    },
  ],
  links: {
    docs: "https://docs.nomos.ms",
  },
};
