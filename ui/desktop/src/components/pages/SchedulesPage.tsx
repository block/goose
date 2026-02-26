import { useNavigate } from 'react-router-dom';
import SchedulesView from '../schedule/SchedulesView';

export default function SchedulesPage() {
  const navigate = useNavigate();
  return <SchedulesView onClose={() => navigate('/')} />;
}
