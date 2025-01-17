export function showToast(message: string, type: 'success' | 'error') {
    const toast = document.createElement('div');
    toast.className = `
        fixed bottom-4 right-4 p-4 
        rounded-lg shadow-lg 
        ${type === 'success' 
            ? 'bg-white dark:bg-gray-800 text-green-600 dark:text-green-400 border border-green-200 dark:border-green-800' 
            : 'bg-white dark:bg-gray-800 text-red-600 dark:text-red-400 border border-red-200 dark:border-red-800'
        }
        transform transition-all duration-300 ease-in-out
        animate-in fade-in slide-in-from-bottom-5
    `;
    toast.textContent = message;
    document.body.appendChild(toast);
    
    // Animate out after 5 seconds instead of 2
    setTimeout(() => {
        toast.style.opacity = '0';
        toast.style.transform = 'translateY(1rem)';
        setTimeout(() => toast.remove(), 300);
    }, 5000);
} 
