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
                <a href="{{url}}">#{{tag}}</a>&nbsp;&nbsp;
                {{/tags}}
            </p>
            <p>
                {{{post_content}}}
            </p>

            {{#comments_enabled}}
            <div id="comments" class="comments">
                <h3>Comments ({{comment_count}})</h3>

                {{#comments}}
                <div class="comment">
                    <h5>{{author}} on {{date}} {{time}}</h5>
                    <p>{{{body_html}}}</p>
                </div>
                {{/comments}}
                {{^comments}}
                <p class="no-comments">No comments yet. Be the first to write one.</p>
                {{/comments}}

                <form class="comment-form" method="post" action="{{comment_action}}">
                    <h3>Leave a comment</h3>
                    {{#comment_posted}}
                    <p class="comment-success">Your comment was published.</p>
                    {{/comment_posted}}
                    {{#comment_error}}
                    <p class="comment-error">{{comment_error}}</p>
                    {{/comment_error}}

                    <label for="comment-author">Name</label>
                    <input id="comment-author" type="text" name="author" maxlength="80" required>

                    <label for="comment-body">Comment</label>
                    <textarea id="comment-body" name="body" rows="6" maxlength="4000" required></textarea>

                    <div class="website-field" aria-hidden="true">
                        <label for="comment-website">Website</label>
                        <input id="comment-website" type="text" name="website" tabindex="-1" autocomplete="off">
                    </div>

                    <div class="captcha">
                        <label for="captcha-answer">{{captcha_question}}</label>
                        <div class="captcha-image">{{{captcha_svg}}}</div>
                        <input type="hidden" name="captcha_id" value="{{captcha_id}}">
                        <input id="captcha-answer" type="text" name="captcha_answer" inputmode="numeric" required>
                    </div>

                    <button type="submit">Publish comment</button>
                </form>
            </div>
            {{/comments_enabled}}

        </div>

    </div>
    <!-- JS highlighter -->
    <script src="/public/prism.js"></script>
</body>

</html>