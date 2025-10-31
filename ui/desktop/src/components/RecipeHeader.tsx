interface RecipeHeaderProps {
  title: string;
}

export function RecipeHeader({ title }: RecipeHeaderProps) {
  return (
    <div className={`flex items-center justify-between px-4 py-2 border-b border-borderSubtle'}`}>
      <div className="flex items-center ml-6">
        <span className="w-2 h-2 rounded-full bg-green-500 mr-2" />
        <span className="text-sm">
          <span className="text-textSubtle">Recipe</span>{' '}
          <span className="text-textStandard">{title}</span>
        </span>
      </div>
    </div>
  );
}
