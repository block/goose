# Blog Post Setup Instructions

## What Has Been Created

1. **Blog Post Content**: A comprehensive blog post has been created at:
   - `blog-post-creating-and-sharing-effective-goose-recipes.md`
   
   This file contains the full blog post content that you can use to publish on your chosen platform (Dev.to, Hashnode, Medium, personal blog, etc.).

2. **Community Content Entry**: An entry has been added to `community-content.json` with placeholders that need to be filled in.

## Next Steps

### Step 1: Publish Your Blog Post

1. Copy the content from `blog-post-creating-and-sharing-effective-goose-recipes.md`
2. Publish it on your chosen platform:
   - [Dev.to](https://dev.to)
   - [Hashnode](https://hashnode.com)
   - [Medium](https://medium.com)
   - Your personal blog
   - Any other platform

3. **Important**: Make sure the published blog post is publicly accessible and has a permanent URL.

### Step 2: Update community-content.json

Open `documentation/src/pages/community/data/community-content.json` and replace the placeholders in the new entry:

```json
{
  "title": "Creating and Sharing Effective goose Recipes",
  "author": {
    "name": "Shreyansh Singh",              // ✅ Already set (update if different)
    "handle": "Shreyanshsingh23"            // ✅ Already set
  },
  "type": "blog",
  "url": "YOUR_BLOG_POST_URL",             // Replace with the actual URL to your published blog post
  "thumbnail": "https://images.unsplash.com/photo-1555949963-aa79dcee981c?w=400&h=225&fit=crop&crop=entropy&auto=format",
  "submittedDate": "YYYY-MM-DD",           // Replace with the date you're submitting (format: 2025-10-XX)
  "tags": [
    "hacktoberfest",
    "goose",
    "recipes"
  ],
  "hacktoberfest": true
}
```

**Fields to update:**
- `name`: ✅ Already set to "Shreyansh Singh" (update if this is incorrect)
- `handle`: ✅ Already set to "Shreyanshsingh23"
- `url`: ⚠️ **REQUIRED** - Replace `YOUR_BLOG_POST_URL` with the actual URL to your published blog post
- `submittedDate`: ⚠️ **REQUIRED** - Replace `YYYY-MM-DD` with the date in YYYY-MM-DD format (e.g., "2025-10-28")
- `thumbnail`: Optional - keep the current Unsplash image or replace with your own (must end with `?w=400&h=225&fit=crop&crop=entropy&auto=format`)

### Step 3: Create Pull Request

1. **Fork the repository** (if not already done):
   - Go to https://github.com/block/goose and click "Fork"

2. **You're already on the correct branch**: `blog-post-goose-recipes`

3. **Commit your changes**:
   ```bash
   git add documentation/src/pages/community/data/community-content.json
   git commit --signoff -m "docs: add blog post 'Creating and Sharing Effective goose Recipes'"
   ```

4. **Push to your fork**:
   ```bash
   git push origin blog-post-goose-recipes
   ```

5. **Create the Pull Request**:
   - Go to https://github.com/block/goose
   - Click "New Pull Request"
   - Select your fork and branch
   - Fill out the PR description including:
     - Your email (for the $10 OpenRouter LLM credits)
     - Link to the published blog post
     - Brief description

6. **Link PR in Issue #4726**:
   - Go to https://github.com/block/goose/issues/4726
   - Comment with a link to your PR

## Validation Checklist

Before submitting:
- [ ] Blog post is published and publicly accessible
- [ ] `url` field in community-content.json is updated (replaces `YOUR_BLOG_POST_URL`)
- [ ] `submittedDate` field is updated (replaces `YYYY-MM-DD`)
- [ ] JSON file is valid (no syntax errors)
- [ ] GitHub handle is set to "Shreyanshsingh23" (✅ already done)
- [ ] Display name is correct (currently "Shreyansh Singh" - update if needed)
- [ ] Blog post URL is correct and accessible
- [ ] Commit message follows conventional commits format
- [ ] Commit includes `--signoff` flag
- [ ] PR description includes your email for credits

## Notes

- The blog post content follows goose brand guidelines (lowercase "g" for "goose")
- The thumbnail uses a relevant Unsplash image; you can replace it with your own if desired
- Make sure your blog post is comprehensive and covers the required topics from issue #4726
- The blog post content is ready to use as-is, or you can customize it to match your writing style

## Need Help?

- Check the [Contributing Guide](CONTRIBUTING.md)
- Review [Contributing Recipes Guide](CONTRIBUTING_RECIPES.md)
- Ask questions in the [goose Discord](https://discord.gg/goose-oss)
- Check existing submissions in `community-content.json` for reference

Good luck with your submission! 🎉

