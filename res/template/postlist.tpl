<html>
<head>
<meta name="viewport" content="maximum-scale=1.0,width=device-width,initial-scale=1.0">
<title>Texted2 example blog</title>
</head>
<body>
    <h1>Welcome!</h1>

    {{#post_list}}
        <p>{{date}} {{time}}</p>
        <h3><a href='{{link}}'>{{title}}</a></h3>
        <span>{{{summary}}}</span>
        <hr />
    {{/post_list}}
</body>
</html>
