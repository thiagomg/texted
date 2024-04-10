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

    <div class="row">
        <div class="card">

            <h2>{{{post_title}}}</h2>
            <h5>Created by {{author}} on {{date}} {{time}}</h5>
            <p>
                <strong>Tags:</strong>
                {{#tags}}
                <a href="/list/{{tag}}/">#{{tag}}</a>&nbsp;&nbsp;
                {{/tags}}
            </p>
            <p>
                {{{post_content}}}
            </p>

        </div>

    </div>
    <!-- JS highlighter -->
    <script src="/public/prism.js"></script>
</body>

</html>