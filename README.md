<a href="https://gitlab.com/thiagomg/texted2"><img height="100%"
src="https://raw.githubusercontent.com/thiagomg/texted/refs/heads/main/Texted-logo.png"></a>

# Texted - Free your text!

Why did I create one blog system instead of using some existing blog platform?

1. I am writing in some other platforms, such as [dev.to/thiagomg](https://dev.to/thiagomg) however if they lose their
   data or close their platform, I don't want to lose what I've written
2. I wanted to learn Rust, therefore I created a blog system
3. If I get the blog posts, with the way they are structured here and add to github/gitlab/etc, it will just work (adding a header to the files)

## Getting started

### Installing

Using brew:

```bash
brew tap thiagomg/texted
brew install texted
```

Or using cargo

```bash
cargo install texted
```

### Bootstrapping a new blog

```bash
# Let's create a new directory for our new blog
mkdir ~/my-awesome-blog && cd ~/my-awesome-blog
# And now create the skeleton of a fully functional blog
texted-tool bootstrap --out-dir .
```

Now, let's run and test it.

```bash
# Inside the same ~/my-awesome-blog directory, run
texted
```

And open in your browser http://127.0.0.1:8001/

### Adding a new post

Texted by default caches all urls and posts for best performance. Please quit texted and run again to see new posts in the listing.
If you want to disable rendering cache to see changes in the post content without restarting, change
`rendering_cache_enabled = false` in the texted.toml configuration file


```bash
# Still inside the same ~/my-awesome-blog directory, run
pushd posts/

# This will create one directory such as 20240428_how_to_write_a_new_blog_post
# with a file index.md inside.
texted-tool post --title "How to write a new blog post" -o dir

# And let's run texted again
popd
texted
```

You now have a new post available in the post list

## Building from sources

How to build?

```bash
cargo build --release
```

How to run?

```bash
cargo run --bin texted
```

## How to personalise my site?

You can create your HTML templates using [mustache templates](https://mustache.github.io). The examples show all
supported fields

Templates can have images, css and js, etc. Those support files should live in the public directory

In the file `texted.toml`, you can configure the location in the keys:

- template_dir
- public_dir

## How to add posts and pages?

Posts live in the directory pointed in the configuration key `posts_dir` and pages in the key `pages_dir`

The only difference between pages and posts are:

1. Pages are rendered using page.tpl template. Posts are rendered using view.tpl template
2. Posts are listed the the list API (`server-address/list`) while pages are not listed

From now on, all said about posts directory structure also is the same for pages

There are 2 ways to add posts. Inside posts configured directory, you can create a mardown file or a directory.

### File posts

Let's say your server is running in your local host, port 8080 (127.0.0.1:8080) and posts are inside the post directory.

File posts are text-only posts (or posts without local images). The file name will be used as url. E.g.

The file `post/post_without_images.md` will be accesible using the url `http://127.0.0.1:8080/view/post_without_images`

### Directory posts

Directories containing a file `index.md` (configurable in the file `texted.toml`), will be treated as a post. E.g.

The directory `post/post_with_image` will be accesible using the url `http://127.0.0.1:8080/view/post_with_image`

## Structure of a post

### Header

Each post or page markdown file contains a header with a unique identification, date and author. It is followed by the
post title. The html comment is optional, but some markdown editors screw up the header if not in an html comment, so I added it in the example

```markdown
<!--
[ID]: # (21c1e9ad-4ebb-4168-a543-fbf77cc35a85)
[DATE]: # (2024-02-12 22:54:00.000)
[AUTHOR]: # (thiago)
-->

# How does it work?
```

Everything after that is the body of the post

### More

In the post list, what is presented is a part of the post body. To determine when it stops, you add the `<!-- more -->`
tag.

See `post_without_images.md` or `post_with_image/index.md` for an example

### Comments

Posts can accept comments. Each post has its own JSONL file (one JSON object per line) in the directory configured by `[comments]`.

```toml
[comments]
dir = "res/comments"
```

Remove the `[comments]` block to disable comments.

The file name matches the post URL. The post `/view/post_without_images/` is stored in `res/comments/post_without_images.jsonl`:

```json
{"id":"c0a1b2d3-e4f5-4678-9abc-def012345678","author":"Ada","date":"2024-03-01 10:15:00","body":"Clear explanation."}
{"id":"d1b2c3d4-e5f6-4789-abcd-ef0123456789","author":"Lin","date":"2024-03-02 18:42:11","body":"Works well with git."}
```

The comment form includes a math captcha. You can style the comments section in `view.tpl` and `simple_flex.css`. The template fields are `comments_enabled`, `comments`, `comment_count`, `captcha_id`, `captcha_question`, `captcha_svg`, `comment_error` and `comment_posted`.

### Tip

If you run the texted-post binary, it will create a post skeleton with header, title and sample body to you.

## Configurable content routes

A route's name determines its URL prefix. Routes have one of three types:

- `page` renders individual pages without comments.
- `post` renders individual posts, with comments when `[comments]` is enabled.
- `postlist` lists content from a named page or post route, with pagination and tag filtering.

For example:

```toml
[routes.doc]
type = "page"
directory = "res/docs"
template = "content.tpl"
title = "Documentation"
navigation = ["introduction", "installation"]

[routes.article]
type = "post"
directory = "res/articles"
template = "content.tpl"
title = "Articles"

[routes.tutorials]
type = "postlist"
source = "doc"
template = "contentlist.tpl"

[routes.blog]
type = "postlist"
source = "article"
template = "postlist.tpl"
```

This publishes `/doc/introduction/`, `/article/hello/`, `/tutorials/`, and `/blog/`.
The bundled resources include these content files and templates; uncomment the corresponding examples in `texted.toml` to try them after bootstrapping.

Content directories follow the existing file and directory page conventions. `directory` uses the same path rules as `[paths]`, including `${exe_dir}`. Templates are relative to `paths.template_dir`. Content discovery is one directory deep; nested content hierarchies are not supported. A directory page's attachments are served at `/prefix/slug/filename`.

A list's `source` references a `page` or `post` route, so its entries and preview images use the source prefix automatically. Multiple lists can reference the same source with different templates. Visit `/blog/?page=2` for pagination or `/blog/rust/` to filter by tag. Empty lists render normally.

### Ordering and navigation

Page routes default to alphabetical title order. Post routes default to newest first. Equal titles or dates are ordered by slug. Optional `navigation` on the source route specifies the exact order and membership of both navigation and listings. Unlisted content remains accessible by URL, without previous/next navigation. An empty `navigation = []` hides all entries from lists. Duplicate or missing navigation entries are configuration errors.

The optional `title` defaults to the route name. Content templates receive these additional fields:

| Field | Meaning |
|---|---|
| `route_name`, `route_title`, `route_url` | Content source prefix, title, and prefix URL |
| `content_url` | Current page/post URL |
| `navigation_pages` | Entries with `title`, `url`, and `current` |
| `previous_page`, `next_page` | Optional neighbouring entries with `title` and `url` |
| `listing_url` | A listing for this content source, or empty if none exists |
| `comment_action` | URL for submitting a comment on the current post |
| `tags` | Existing `tag` field plus its listing `url`, or empty URL without a listing |

The source prefix URL (`route_url`) is metadata; it serves an index only if that prefix is configured as a `postlist`. Use `listing_url` for an index link. When multiple lists reference a source, content pages choose the alphabetically first listing prefix, preferring `list` for the legacy `view` source.

For example, a sidebar can use:

```html
<nav>
  {{#listing_url}}<a href="{{listing_url}}">All pages</a>{{/listing_url}}
  {{#navigation_pages}}
    <a href="{{url}}" {{#current}}aria-current="page"{{/current}}>{{title}}</a>
  {{/navigation_pages}}
</nav>
{{#previous_page}}<a href="{{url}}">Previous: {{title}}</a>{{/previous_page}}
{{#next_page}}<a href="{{url}}">Next: {{title}}</a>{{/next_page}}
```

List templates retain `post_list` (`title`, `link`, `summary`, `date`, `time`), `tags`, `page_list`, and `show_pagination`. Tag entries expose `tag` and `url`; pagination entries expose `number`, `current`, and `url`. Source route metadata and navigation are also available. `listing_url` refers to the current list. Use these generated URLs instead of hardcoding `/view` or `/list` in custom templates. Use `action="{{comment_action}}"` for comment forms. Existing `link` on an individual post remains its slug.

### Existing blogs, comments, and RSS

Without `[routes]`, Texted supplies the existing routes:

- `page`: `paths.pages_dir`, `page.tpl`.
- `view`: `paths.posts_dir`, `view.tpl`.
- `list`: source `view`, `postlist.tpl`.

Explicit definitions replace defaults with the same name. Other defaults remain active. Existing URLs and templates continue to work, including `/list` without a trailing slash. Prefixes must contain only ASCII letters, digits, hyphens, or underscores; `public` and `rss` are reserved. Referenced directories and templates must exist at startup, even for empty routes.

Existing `view` comments retain their filenames. Custom post route comments live in `comments.dir/routes/prefix/slug.jsonl`, keeping identical slugs in separate blogs independent. Page routes do not accept comments.

To select a different post route for RSS, add `source = "article"` inside `[rss_feed]`. It defaults to `view` and must reference a post route. The homepage post count follows this source, or `view` if RSS is disabled. The homepage itself continues to use `index.tpl`.

Route definitions and content membership are loaded at startup. Restart Texted after changing configuration or adding/removing content. With `rendering_cache_enabled = false`, edits to existing content, navigation titles, and templates appear without restarting. With caching enabled, restart to refresh content and navigation. Rendered comment forms and comment feedback are never cached.
