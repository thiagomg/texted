<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width,initial-scale=1">
    <title>{{route_title}}</title>
    <link rel="stylesheet" href="/public/simple_flex.css">
</head>
<body>
    <header><a href="/">Home</a><h1>{{route_title}}</h1></header>
    <nav aria-label="Tags">
        <a href="{{listing_url}}">All</a>
        {{#tags}}<a href="{{url}}">#{{tag}}</a> {{/tags}}
    </nav>
    <main>
        {{#post_list}}
        <article class="card">
            <h2><a href="{{link}}">{{title}}</a></h2>
            {{{summary}}}
            <a href="{{link}}">Read more</a>
        </article>
        {{/post_list}}
        {{^post_list}}<p>No content found.</p>{{/post_list}}
    </main>
    {{#show_pagination}}
    <nav aria-label="Pagination">
        {{#page_list}}
        <a href="{{url}}" {{#current}}aria-current="page"{{/current}}>{{number}}</a>
        {{/page_list}}
    </nav>
    {{/show_pagination}}
</body>
</html>
