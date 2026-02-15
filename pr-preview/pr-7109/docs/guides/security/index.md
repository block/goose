<h1 className={styles.pageTitle}>Staying Safe with goose</h1>
<p className={styles.pageDescription}>
  Understanding security risks and following best practices can help you secure your workflows with goose.
</p>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 Documentation & Guides</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="Prompt Injection Detection"
      description="Detect and prevent potentially harmful commands before they run."
      link="/docs/guides/security/prompt-injection-detection"
    />
    <Card 
      title="Classification API Specification"
      description="API specification for self-hosting ML-based prompt injection detection endpoints."
      link="/docs/guides/security/classification-api-spec"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📝 Featured Blog Posts</h2>
  <div className={styles.cardGrid}>
    <Card      
      title="How to Determine If An MCP Server Is Safe"
      description="Before you plug your agent into just any MCP server, here's how to check if it's actually safe."
      link="/blog/2025/03/26/mcp-security"
    />
    <Card      
      title="Securing the Model Context Protocol"
      description="Building secure and capable AI integrations with Model Context Protocol (MCP) at Block."
      link="/blog/2025/03/31/securing-mcp"
    />
  </div>
</div>