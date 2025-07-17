import React from 'react';

interface GooseBuiltinInstallerProps {
  extensionName: string;
  description?: string;
}

const GooseBuiltinInstaller: React.FC<GooseBuiltinInstallerProps> = ({
  extensionName,
  description
}) => {
  return (
    <div className="goose-builtin-installer">
      <div className="installation-steps">
        <h4>Enable {extensionName} Extension</h4>
        {description && (
          <p className="extension-description">{description}</p>
        )}
        
        <div className="step-list">
          <div className="step">
            <span className="step-number">1</span>
            <span className="step-text">Click <code>...</code> in the upper right corner</span>
          </div>
          
          <div className="step">
            <span className="step-number">2</span>
            <span className="step-text">Click <code>Advanced Settings</code></span>
          </div>
          
          <div className="step">
            <span className="step-number">3</span>
            <span className="step-text">Under <code>Extensions</code>, toggle <code>{extensionName}</code> to on</span>
          </div>
        </div>
      </div>
      
      <style jsx>{`
        .goose-builtin-installer {
          border: 1px solid var(--ifm-color-emphasis-300);
          border-radius: 8px;
          padding: 1.5rem;
          margin: 1rem 0;
          background: var(--ifm-background-surface-color);
        }
        
        .goose-builtin-installer h4 {
          margin: 0 0 0.5rem 0;
          color: var(--ifm-color-primary);
          font-size: 1.1rem;
        }
        
        .extension-description {
          margin: 0 0 1rem 0;
          color: var(--ifm-color-emphasis-700);
          font-style: italic;
        }
        
        .step-list {
          display: flex;
          flex-direction: column;
          gap: 0.75rem;
        }
        
        .step {
          display: flex;
          align-items: flex-start;
          gap: 0.75rem;
        }
        
        .step-number {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 24px;
          height: 24px;
          background: var(--ifm-color-primary);
          color: white;
          border-radius: 50%;
          font-size: 0.875rem;
          font-weight: 600;
          flex-shrink: 0;
          margin-top: 2px;
        }
        
        .step-text {
          flex: 1;
          line-height: 1.5;
        }
        
        .step-text code {
          background: var(--ifm-code-background);
          padding: 0.125rem 0.25rem;
          border-radius: 3px;
          font-size: 0.875em;
        }
        
        @media (max-width: 768px) {
          .goose-builtin-installer {
            padding: 1rem;
          }
          
          .step {
            gap: 0.5rem;
          }
          
          .step-number {
            width: 20px;
            height: 20px;
            font-size: 0.75rem;
          }
        }
      `}</style>
    </div>
  );
};

export default GooseBuiltinInstaller;
