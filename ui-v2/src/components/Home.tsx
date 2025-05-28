import GooseLogo from '../components/GooseLogo';

export default function Home() {
  return (
    <div className="flex items-center gap-4 mb-4">
      <GooseLogo />
      <h1 className="text-2xl font-bold text-textProminent">Goose v2</h1>
    </div>
  );
}
