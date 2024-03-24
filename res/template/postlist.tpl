<html>
<head>
<meta name="viewport" content="maximum-scale=1.0,width=device-width,initial-scale=1.0">
<title>Texted2 example blog</title>
</head>
<body>
    <h1>Welcome!</h1>

    Tags:
    {{#tags}}
    <a href="/list/{{tag}}/">#{{tag}}</a>
    {{/tags}}

    {{#post_list}}
        <p>{{date}} {{time}}</p>
        <h3><a href='{{link}}'>{{title}}</a></h3>
        <span>{{{summary}}}</span>
        <hr />
    {{/post_list}}

    {{#show_pagination}}
    <div>
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


</body>
</html>
