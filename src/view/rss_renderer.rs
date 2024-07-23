use std::io::Cursor;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use crate::content::{Content, PostId};

/* Example
<?xml version="1.0" encoding="UTF-8" ?>
<rss version="2.0">

<channel>
  <title>Thiago Cafe blog posts</title>
  <link>https://thiagocafe.com</link>
  <description>This blog is about programming and other technological things. Written by someone developing software for fun and professionally for longer than I want to admit and in more programming languages that I can remember</description>
  <item>
    <title>Creating a daemon in System D</title>
    <link>https://thiagocafe.com/view/20240216_creating_a_daemon_in_systemd</link>
    <description>So, you created your awesome server-side application and you are ready to start using</description>
  </item>
  <item>
    <title>What I learned after 20+ years of software development</title>
    <link>https://thiagocafe.com/view/20220402_what_i_learned</link>
    <description>How to be a great software engineer?</description>
  </item>
</channel>

</rss>
*/

pub struct RssChannel<'a> {
    pub ch_title: &'a str,
    pub ch_link: &'a str,
    pub ch_desc: &'a str,
}


impl<'a> RssChannel<'a> {
    pub fn render(&self, contents: &[Arc<Content>]) -> quick_xml::Result<Vec<u8>> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        // <?xml version="1.0" encoding="UTF-8" ?>
        let decl = Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None));
        writer.write_event(decl)?;

        // <rss version="2.0">
        let mut rss = BytesStart::new("rss");
        rss.push_attribute(("version", "2.0"));
        writer.write_event(Event::Start(rss))?;

        // <channel>
        writer.write_event(Event::Start(BytesStart::new("channel")))?;

        // <title>Thiago Cafe blog posts</title>
        push_text(&mut writer, "title", self.ch_title)?;

        // <link>https://thiagocafe.com</link>
        push_text(&mut writer, "link", self.ch_link)?;

        // <description>This blog is about programming and other technological things. Written by someone developing software for fun and professionally for longer than I want to admit and in more programming languages that I can remember</description>
        push_text(&mut writer, "description", self.ch_desc)?;

        for content in contents {
            // <item>
            writer.write_event(Event::Start(BytesStart::new("item")))?;

            // <title>What I learned after 20+ years of software development</title>
            let title = content.title.as_str();
            push_text(&mut writer, "title", title)?;

            // <link>https://thiagocafe.com/view/20220402_what_i_learned</link>
            let link = full_link(self.ch_link, content.link.as_str());
            push_text(&mut writer, "link", link.as_str())?;

            // <guid isPermaLink="false">https://thiagocafe.com/view/20220402_what_i_learned</guid>
            let PostId(ref guid) = content.header.id;
            let mut guid_elem = BytesStart::new("guid");
            guid_elem.push_attribute(("isPermaLink", "false"));
            writer.write_event(Event::Start(guid_elem))?;
            writer.write_event(Event::Text(BytesText::new(guid.as_str())))?;
            writer.write_event(Event::End(BytesEnd::new("guid")))?;

            // <description>How to be a great software engineer?</description>
            let description = content.rendered.as_str();
            push_cdata(&mut writer, "description", description)?;

            // <pubDate>Wed, 20 Apr 2022 16:00:00 +0200</pubDate>
            let dt = &content.header.date;
            let dt = TimeZone::from_utc_datetime(Utc::now().offset(), dt);
            push_text(&mut writer, "pubDate", &dt.to_rfc2822())?;


            // </item>
            writer.write_event(Event::End(BytesEnd::new("item")))?;
        }

        // </channel>
        writer.write_event(Event::End(BytesEnd::new("channel")))?;
        // </rss>
        writer.write_event(Event::End(BytesEnd::new("rss")))?;

        Ok(writer.into_inner().into_inner())
    }
}

fn full_link(base_url: &str, link: &str) -> String {
    let base_url = if base_url.ends_with('/') {
        base_url.to_string()
    } else {
        format!("{}/", base_url)
    };

    let link = if link.ends_with('/') {
        link.to_string()
    } else {
        format!("{}/", link)
    };

    format!("{}view/{}", base_url, link)
}

fn push_text(writer: &mut Writer<Cursor<Vec<u8>>>, tag: &str, text: &str) -> quick_xml::Result<()> {
    writer.write_event(Event::Start(BytesStart::new(tag)))?;
    writer.write_event(Event::Text(BytesText::new(text)))?;
    writer.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

fn push_cdata(writer: &mut Writer<Cursor<Vec<u8>>>, tag: &str, text: &str) -> quick_xml::Result<()> {
    writer.write_event(Event::Start(BytesStart::new(tag)))?;
    if text.contains("]]>") {
        let new_text = text.replace("]]>", "]] >");
        writer.write_event(Event::CData(BytesCData::new(&new_text)))?;
    } else {
        writer.write_event(Event::CData(BytesCData::new(text)))?;
    }
    writer.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::str;
    use std::sync::Arc;

    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use crate::content::{Content, ContentHeader, PostId};

    use super::*;

    fn create_cont(id: &str) -> Arc<Content> {
        let dt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 01, 02).unwrap(),
            NaiveTime::from_hms_opt(5, 6, 7).unwrap(),
        );
        let content = Content {
            header: ContentHeader {
                file_name: PathBuf::from(format!("post-{}.md", id)),
                id: PostId(id.to_string()),
                date: dt,
                author: "Thiago".to_string(),
                tags: vec![format!("first-tag-{}", id), format!("second-tag-{}", id)],
            },
            link: format!("post-{}", id),
            title: format!("title-of-post-{}", id),
            rendered: format!("summary-of-post-{}", id),
        };

        Arc::new(content)
    }

    #[test]
    fn render_xml() {
        let contents = vec![create_cont("1"), create_cont("2")];

        let ch_title = "my feed";
        let ch_link = "https://thiagocafe.com";
        let ch_desc = "My blog feed";
        let rss = RssChannel {
            ch_title,
            ch_link,
            ch_desc,
        };
        let xml = rss.render(&contents).unwrap();
        println!("XML: {}", str::from_utf8(&xml).unwrap());
        assert_eq!(str::from_utf8(&xml).unwrap(), EXPECTED);
    }
    
    const EXPECTED: &str = r##"<?xml version="1.0" encoding="UTF-8"?><rss version="2.0"><channel><title>my feed</title><link>https://thiagocafe.com</link><description>My blog feed</description><item><title>title-of-post-1</title><link>https://thiagocafe.com/view/post-1/</link><guid isPermaLink="false">1</guid><description><![CDATA[summary-of-post-1]]></description><pubDate>Tue, 2 Jan 2024 05:06:07 +0000</pubDate></item><item><title>title-of-post-2</title><link>https://thiagocafe.com/view/post-2/</link><guid isPermaLink="false">2</guid><description><![CDATA[summary-of-post-2]]></description><pubDate>Tue, 2 Jan 2024 05:06:07 +0000</pubDate></item></channel></rss>"##;
}