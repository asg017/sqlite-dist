import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "sqlite-dist",
  description: "A tool for distributing SQLite extensions through npm, pip, gem, Github, andmore!",
  base: '/sqlite-dist/',
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Reference', link: '/config' }
    ],

    sidebar: [
      {
        //text: 'sqlite-dist',
        items: [
          { text: 'Introduction', link: '/intro' },
          { text: 'sqlite-dist.toml Reference', link: '/config' }
        ],
      },
      {
        text: "Targets",
        items: [
          { text: 'Node.js (npm)', link: '/targets/npm' },
          { text: 'Python (pip)', link: '/targets/pip' },
          { text: 'Ruby (gem)', link: '/targets/gem' },
          { text: 'GitHub Releases', link: '/targets/github-releases' },
          {text: "Datasette", link: "/targets/datasette" },
          {text: "sqlite-utils", link: "/targets/sqlite-utils" },
          {text: "sqlpkg", link: "/targets/sqlpkg" },
          {text: "SPM", link: "/targets/spm" },
          {text: "Amalgamation", link: "/targets/amalgamation" }

        ]
      }
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/asg017/sqlite-dist' }
    ]
  }
})
