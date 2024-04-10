<html>

<head>
    <meta name="viewport" content="maximum-scale=1.0,width=device-width,initial-scale=1.0">
    <title>Texted</title>
    <link href="/public/prism.css" rel="stylesheet" />
</head>

<body>
    <link rel="stylesheet" type="text/css" href="/public/simple_flex.css">

    <div class="header">
        Texted - Free your text!</span>
    </div>

    <div class="row">
        <div class="card">
            <h2>Who's talking?</h2>
            <p>
            I work as a software developer for {{years_developing}} years!
            </p>

            <h2>Some stats</h2>
            <ul>
                <li>Number of posts: {{post_count}}</li>
                <li>Number of days since I started this blog: {{days_since_started}}</li>
            </ul>

        </div>
        <div class="card">
            <p><a href="list">List of posts</a></p>
            <p>Pages are supported too:
            <a href="page/bio">My bio</a>
            </p>
        </div>
        <div class="card">

            <p>Template images should be added in the public directory</p>
            <img src="public/dino.jpg">

        </div>

    </div>


</body>

</html>