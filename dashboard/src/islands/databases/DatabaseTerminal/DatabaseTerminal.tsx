import TerminalTab from '@islands/app-detail/TerminalTab/TerminalTab';

type Props = {
  dbId: string;
};

// IF-165: a terminal into a managed database container. Reuses the app terminal
// component, pointing it at the database WebSocket endpoint and showing the
// live-database warning. The backend launches the engine's native client
// (psql / mysql / mongosh / redis-cli / …) instead of a plain shell.
export default function DatabaseTerminal({ dbId }: Props) {
  return (
    <TerminalTab
      appId={dbId}
      wsPath={`/databases/${dbId}/terminal`}
      warning="You are connected to a live database. Commands execute immediately."
      emptyHint="Open a client session into your database container. The database must be running."
    />
  );
}
