import { useState } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import { Check, X, MessageSquare } from 'lucide-preact';
import Input from '@islands/shared/Input/Input';
import styles from './approval-badge.module.css';

type Props = {
  deployId: string;
  status: string;
  requiresApproval: boolean;
};

export default function ApprovalBadge({ deployId, status, requiresApproval }: Props) {
  const [approving, setApproving] = useState(false);
  const [rejecting, setRejecting] = useState(false);
  const [showComment, setShowComment] = useState(false);
  const [comment, setComment] = useState('');
  const [resolved, setResolved] = useState<'approved' | 'rejected' | null>(null);

  if (!requiresApproval || status !== 'pending') return null;
  if (resolved) {
    return (
      <span
        class={`${styles.badge} ${resolved === 'approved' ? styles.approved : styles.rejected}`}
        role="status"
        aria-live="polite"
      >
        {resolved === 'approved' ? 'Approved' : 'Rejected'}
      </span>
    );
  }

  async function handleApprove() {
    setApproving(true);
    try {
      await api.approveDeploy(deployId, comment || undefined);
      setResolved('approved');
      addToast('success', 'Deploy approved');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to approve deploy');
    }
    setApproving(false);
  }

  async function handleReject() {
    setRejecting(true);
    try {
      await api.rejectDeploy(deployId, comment || undefined);
      setResolved('rejected');
      addToast('info', 'Deploy rejected');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to reject deploy');
    }
    setRejecting(false);
  }

  return (
    <div class={styles.container}>
      {/* a11y [WCAG 4.1.3]: status announced to AT */}
      <span class={`${styles.badge} ${styles.pending}`} role="status" aria-live="polite">
        Awaiting Approval
      </span>
      <div class={styles.actions}>
        {showComment && (
          <Input
            name="approval-comment"
            value={comment}
            onChange={setComment}
            placeholder="Optional comment..."
            label="Approval comment"
            className={styles.commentInput}
          />
        )}
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowComment(!showComment)}
          aria-label={showComment ? 'Hide comment input' : 'Add a comment'}
        >
          <MessageSquare size={14} aria-hidden="true" />
        </Button>
        <Button
          variant="primary"
          size="sm"
          onClick={handleApprove}
          loading={approving}
          disabled={rejecting}
          aria-label="Approve deploy"
        >
          <Check size={14} aria-hidden="true" /> Approve
        </Button>
        <Button
          variant="danger"
          size="sm"
          onClick={handleReject}
          loading={rejecting}
          disabled={approving}
          aria-label="Reject deploy"
        >
          <X size={14} aria-hidden="true" /> Reject
        </Button>
      </div>
    </div>
  );
}
