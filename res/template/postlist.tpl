<!DOCTYPE html>
<html>
<head>
    <meta name="viewport" content="maximum-scale=1.0,width=device-width,initial-scale=1.0">
    <title>Texted</title>
    <link href="/public/prism.css" rel="stylesheet" />
</head>
<body>
    <link rel="stylesheet" type="text/css" href="/public/simple_flex.css">

    <div class="header">
        <a href="/">Texted</a> - Free your text!</span>
    </div>

    <div id="mini-bio" class="card">
      <h2>About this blog</h2>
      <p>
        This blog contains a list of posts and documents that I wrote myself
      </p>
    </div>

    <div class="row">
      <div class="leftcolumn">
        {{#post_list}}
        <div class="card">
          <h2><a href='{{link}}'>{{title}}</a></h2>
          <h5>(Posted {{date}} {{time}})</h5>
          <p>{{{summary}}}</p>
          <p>... more ...</p>
        </div>
        {{/post_list}}

        {{#show_pagination}}
        <div class="card">
        Pages:
        {{#page_list}}
          {{#current}}
            {{number}}
          {{/current}}
          {{^current}}
            <a href="?page={{number}}">{{number}}</a>
          {{/current}}
        {{/page_list}}
        </div>
        {{/show_pagination}}
    </div>

    <div class="rightcolumn">
      <div id="bio" class="card">
        <h2>About this blog</h2>
        <p>
          This blog contains a list of posts and documents that I wrote myself
        </p>
      </div>
      <div class="card tag-list">
        <h3>List of tags</h3>
        <ul>
          <li><a href="/list">all</a></li>
          {{#tags}}<li><a href="/list/{{tag}}/">#{{tag}}</a></li>{{/tags}}
        </ul>
      </div>
          </div>
</div>

</body>
</html>
