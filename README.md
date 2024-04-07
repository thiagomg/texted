  <a href="https://gitlab.com/thiagomg/texted2"><img height="100%"
   src="https://gitlab.com/thiagomg/texted2/-/raw/62124a2716d0f702f42d2700788c6528e71f6af1/Texted-logo.png"></a>

# Texted 2 - Free your text!

Why did I create one blog system instead of using some existing blog platform?

1. I am writing in some other platforms, such as [dev.to/thiagomg](https://dev.to/thiagomg) however if they lose their data or close their platform, I don't want to lose what I've written
2. I wanted to learn Rust, therefore I created a blog system
3. If I get the blog posts, with the way they are structured here and add to github/gitlab/etc, it will just work.

## Getting started

Installing

```bash
cargo install texted2
```

Generating a sample configuration

```bash
# Generates a config file in the current directory
texted2 --generate-cfg -c texted2.toml

# Or to generate in the default config directory
texted2 --generate-cfg
```

### Building from sources

How to build?

```bash
cargo build --release
```

How to run?

```bash
cargo run --bin texted2
```

## How to personalise my site?

You can create your HTML templates using [mustache templates](https://mustache.github.io). The examples show all supported fields

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

Each post or page markdown file contains a header with a unique identification, date and author. It is followed by the post title.

```
[ID]: # (21c1e9ad-4ebb-4168-a543-fbf77cc35a85)
[DATE]: # (2024-02-12 22:54:00.000)
[AUTHOR]: # (thiago)

# How does it work?
```

Everything after that is the body of the post

### More

In the post list, what is presented is a part of the post body. To determine when it stops, you add the `<!-- more -->` tag.

See `post_without_images.md` or `post_with_image/index.md` for an example

### Tip

If you run the texted2-post binary, it will create a post skeleton with header, title and sample body to you.
