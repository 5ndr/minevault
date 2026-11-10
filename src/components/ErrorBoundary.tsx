import { Component, ErrorInfo, ReactNode } from 'react';
import { AlertTriangle, RotateCcw } from 'lucide-react';

interface Props {
  children: ReactNode;
  fallbackTitle?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('ErrorBoundary caught unhandled component error:', error, errorInfo);
  }

  private handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  public render() {
    if (this.state.hasError) {
      return (
        <div className="card m-4 p-6 border-danger">
          <div className="flex items-center gap-3 text-danger mb-3">
            <AlertTriangle size={24} />
            <h3 className="font-semibold text-lg">
              {this.props.fallbackTitle || 'A rendering error occurred in this view'}
            </h3>
          </div>
          <p className="text-muted text-sm mb-4">
            {this.state.error?.message || 'An unexpected error prevented this view from rendering.'}
          </p>
          <button onClick={this.handleReset} className="btn-secondary flex items-center gap-2">
            <RotateCcw size={14} />
            <span>Try Again</span>
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
