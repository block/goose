use mcp_core::{Content, ToolError};
use serde_json::Value;
use std::fs;

const TEMPLATE: &str = r#"<html>
<head>
    <title>HTML and CSS Slideshow</title>
    <style>
        body {
            font-family: Helvetica, sans-serif;
            padding: 5%;
            text-align: center;
            font-size: 16px;
        }

        /* Styling the area of the slides */
        #slideshow {
            overflow: hidden;
            height: 510px;
            width: 728px;
            margin: 0 auto;
            position: relative;
        }

        /* Style each of the sides with a fixed width and height */
        .slide {
            float: left;
            height: 510px;
            width: 728px;
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            padding: 20px;
            box-sizing: border-box;
        }

        /* Add animation to the slides */
        .slide-wrapper {
            /* Calculate the total width on the basis of number of slides */
            width: calc(728px * var(--num-slides));
            transition: margin-left 0.3s ease-in-out;
        }

        /* Set the background color of each of the slides */
        .slide:nth-child(1) { background: #4CAF50; }  /* Material Green */
        .slide:nth-child(2) { background: #2196F3; }  /* Material Blue */
        .slide:nth-child(3) { background: #FFC107; }  /* Material Amber */

        /* Style slide content */
        .slide h1 {
            color: white;
            font-size: 2.5em;
            margin-bottom: 0.5em;
            text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.2);
        }

        .slide p {
            color: white;
            font-size: 1.5em;
            line-height: 1.4;
            text-shadow: 1px 1px 2px rgba(0, 0, 0, 0.1);
            max-width: 90%;
            margin: 0.5em auto;
        }

        .slide ul, .slide ol {
            color: white;
            font-size: 1.2em;
            text-align: left;
            margin: 1em auto;
            text-shadow: 1px 1px 2px rgba(0, 0, 0, 0.1);
            max-width: 90%;
        }

        .slide li {
            margin-bottom: 0.5em;
        }

        .slide ul ul, .slide ol ol {
            font-size: 0.9em;
            margin: 0.5em 0 0.5em 1em;
        }

        .slide pre {
            font-size: 1.1em;
            text-align: left;
            background: rgba(255, 255, 255, 0.9);
            padding: 1em;
            border-radius: 5px;
            max-width: 90%;
            overflow-x: auto;
        }

        .nav-hint {
            position: fixed;
            bottom: 20px;
            left: 50%;
            transform: translateX(-50%);
            background: rgba(0, 0, 0, 0.7);
            color: white;
            padding: 10px 20px;
            border-radius: 5px;
            font-size: 16px;
            opacity: 1;
            transition: opacity 0.5s;
        }

        .nav-hint.fade {
            opacity: 0;
        }
    </style>
</head>
<body>
    <!-- Define the slideshow container -->
    <div id="slideshow">
        <div class="slide-wrapper" style="--num-slides: 2">
            <!-- First slide -->
            <div class="slide">
                <h1>Your Presentation</h1>
                <p>Use arrow keys to navigate</p>
            </div>

            <!-- SLIDE_TEMPLATE (do not remove this comment)

            <div class="slide">
                <h1>New Slide Title</h1>
                <p>Slide content goes here, can use rich like below:</p>
                <ul>
                    <li>Use make_presentation to:
                        <ul>
                            <li>create - Create new presentation</li>
                            <li>add_slide - Add a new slide with content</li>
                        </ul>
                    </li>
                    <li>For manual edits:
                        <ul>
                            <li>Use developer tools to edit the HTML</li>
                            <li>Update --num-slides in slide-wrapper</li>
                            <li>Copy template below for new slides</li>
                        </ul>
                    </li>
                </ul>
            </div>            

            END_SLIDE_TEMPLATE -->

            <!-- ADD_SLIDES_HERE (do not remove this comment) -->
        </div>
    </div>

    <div class="nav-hint">
        Use ← and → arrow keys to navigate
    </div>

    <script>
        const slideWrapper = document.querySelector('.slide-wrapper');
        const slideWidth = 728;
        let currentSlide = 0;
        const hint = document.querySelector('.nav-hint');
        const totalSlides = document.querySelectorAll('.slide').length;

        // Hide hint after 5 seconds
        setTimeout(() => {
            hint.classList.add('fade');
        }, 5000);

        document.addEventListener('keydown', (e) => {
            if (e.key === 'ArrowLeft') {
                if (currentSlide > 0) {
                    currentSlide--;
                    updateSlide();
                }
            } else if (e.key === 'ArrowRight') {
                if (currentSlide < totalSlides - 1) {
                    currentSlide++;
                    updateSlide();
                }
            }
        });

        function updateSlide() {
            slideWrapper.style.marginLeft = `-${currentSlide * slideWidth}px`;
        }
    </script>
</body>
</html>"#;

pub async fn make_presentation(
    path: &str,
    operation: &str,
    params: Option<&Value>,
) -> Result<Vec<Content>, ToolError> {
    match operation {
        "create" => {
            // Get title from params or use default
            let title = params
                .and_then(|p| p.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("Your Presentation");

            // Replace title in template
            let content = TEMPLATE.replace("Your Presentation", title);

            // Create a new presentation with the template
            fs::write(path, content).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to create presentation file: {}", e))
            })?;

            Ok(vec![Content::text(format!(
                "Created new presentation with title '{}' at: {}\nYou can open it with the command: `open {}` to show user. You should look at the html and consider if you want to ask user if they need to adjust it, colours, typeface and so on.",
                title, path, path
            ))])
        }
        "add_slide" => {
            let content = params
                .and_then(|p| p.get("content"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ToolError::InvalidParameters("Missing 'content' parameter for slide".into())
                })?;

            // Read existing file
            let mut html = fs::read_to_string(path).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to read presentation file: {}", e))
            })?;

            // Find the marker comment
            let marker = "<!-- ADD_SLIDES_HERE";
            let insert_pos = html.find(marker).ok_or_else(|| {
                ToolError::ExecutionError("Invalid presentation file format".into())
            })?;

            // Count actual slides (excluding template)
            let current_slides = html.matches("class=\"slide\"").count() - 1; // -1 for template
            let new_count = current_slides + 1;

            // Update the num-slides value
            html = html.replace(
                &format!("--num-slides: {}", current_slides),
                &format!("--num-slides: {}", new_count),
            );

            // Create new slide HTML
            let slide_html = format!(
                r#"            <div class="slide">
                <h1>{}</h1>
            </div>

            {}"#,
                content, marker
            );

            // Insert the new slide
            html.replace_range(insert_pos..insert_pos + marker.len(), &slide_html);

            // Save the file
            fs::write(path, html).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to update presentation file: {}", e))
            })?;

            Ok(vec![Content::text(format!(
                "Added new slide to presentation. You can view it with: open {}\nNote: when creating, or adding a slide, if the content for a slide is long, edit it so that it uses appropriate size, formatting, lists etc (and can even split it to other slides if needed).",
                path
            ))])
        }
        _ => Err(ToolError::InvalidParameters(format!(
            "Invalid operation: {}. Valid operations are: create, add_slide",
            operation
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_presentation() {
        let test_dir = tempfile::tempdir().unwrap();
        let test_path = test_dir.path().join("test_presentation.html");
        let path_str = test_path.to_str().unwrap();

        // Test default title
        let result = make_presentation(path_str, "create", None).await;
        assert!(result.is_ok(), "Should successfully create presentation");

        // Verify the file exists and contains the default title
        assert!(test_path.exists(), "Presentation file should exist");
        let content = fs::read_to_string(&test_path).unwrap();
        assert!(
            content.contains("Your Presentation"),
            "Should contain default title"
        );

        // Test custom title
        let test_path2 = test_dir.path().join("test_presentation2.html");
        let path_str2 = test_path2.to_str().unwrap();
        let params = serde_json::json!({
            "title": "Custom Title Test"
        });
        let result = make_presentation(path_str2, "create", Some(&params)).await;
        assert!(
            result.is_ok(),
            "Should successfully create presentation with custom title"
        );

        // Verify custom title
        let content = fs::read_to_string(&test_path2).unwrap();
        assert!(
            content.contains("Custom Title Test"),
            "Should contain custom title"
        );
        assert!(
            content.contains("SLIDE_TEMPLATE"),
            "Should contain slide template"
        );
        assert!(
            content.contains("ADD_SLIDES_HERE"),
            "Should contain slides marker"
        );

        // Clean up
        test_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_add_slide() {
        let test_dir = tempfile::tempdir().unwrap();
        let test_path = test_dir.path().join("test_presentation.html");
        let path_str = test_path.to_str().unwrap();

        // First create the presentation
        let result = make_presentation(path_str, "create", None).await;
        assert!(result.is_ok(), "Should successfully create presentation");

        // Add a new slide
        let params = serde_json::json!({
            "content": "New Test Slide"
        });
        let result = make_presentation(path_str, "add_slide", Some(&params)).await;
        assert!(result.is_ok(), "Should successfully add slide");

        // Verify the content
        let content = fs::read_to_string(&test_path).unwrap();
        assert!(
            content.contains("New Test Slide"),
            "Should contain new slide content"
        );
        // Initial template has 1 slide + new slide = 2
        assert!(
            content.contains("--num-slides: 2"),
            "Should have correct slide count"
        );
        assert!(
            content.contains("ADD_SLIDES_HERE"),
            "Should preserve marker"
        );

        // Clean up
        test_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_add_slide_without_content() {
        let test_dir = tempfile::tempdir().unwrap();
        let test_path = test_dir.path().join("test_presentation.html");
        let path_str = test_path.to_str().unwrap();

        // Create the presentation first
        let _ = make_presentation(path_str, "create", None).await;

        // Try to add slide without content
        let result = make_presentation(path_str, "add_slide", None).await;
        assert!(result.is_err(), "Should fail without content");
        match result {
            Err(ToolError::InvalidParameters(msg)) => {
                assert!(msg.contains("Missing 'content' parameter"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }

        // Clean up
        test_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        let result = make_presentation("test.html", "invalid", None).await;
        assert!(result.is_err(), "Should fail with invalid operation");
        match result {
            Err(ToolError::InvalidParameters(msg)) => {
                assert!(msg.contains("Valid operations are: create, add_slide"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }
}
