import React, { useEffect, useState } from 'react';
import { View } from '../../App';
import { ScheduledJob, listSchedules, createSchedule, deleteSchedule } from '../../schedule';
import { Input } from '../ui/input';
import { Button } from '../ui/button';
import BackButton from '../ui/BackButton';

interface SchedulesViewProps {
  setView: (view: View) => void;
}

export default function SchedulesView({ setView }: SchedulesViewProps) {
  const [jobs, setJobs] = useState<ScheduledJob[]>([]);
  const [id, setId] = useState('');
  const [source, setSource] = useState('');
  const [cron, setCron] = useState('');

  const refresh = async () => {
    try {
      setJobs(await listSchedules());
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const handleCreate = async () => {
    try {
      await createSchedule({ id, recipe_source: source, cron });
      setId('');
      setSource('');
      setCron('');
      refresh();
    } catch (e) {
      console.error(e);
    }
  };

  const handleDelete = async (jobId: string) => {
    try {
      await deleteSchedule(jobId);
      refresh();
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="p-4 space-y-4">
      <BackButton onClick={() => setView('chat')} />
      <div className="space-y-2">
        <Input placeholder="Job ID" value={id} onChange={e => setId(e.target.value)} />
        <Input placeholder="Recipe Source" value={source} onChange={e => setSource(e.target.value)} />
        <Input placeholder="Cron Expression" value={cron} onChange={e => setCron(e.target.value)} />
        <Button onClick={handleCreate}>Create</Button>
      </div>
      <ul className="space-y-2">
        {jobs.map(job => (
          <li key={job.id} className="flex justify-between items-center border rounded p-2">
            <div>
              <div className="font-bold">{job.id}</div>
              <div className="text-xs">{job.cron}</div>
            </div>
            <Button variant="outline" onClick={() => handleDelete(job.id)}>Delete</Button>
          </li>
        ))}
      </ul>
    </div>
  );
}
