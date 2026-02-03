package state

// State represents the state machine states.
type State int

const (
	StateIdle State = iota
	StateLoading
	StateLoaded
	StateError
)

// StateContext holds the state machine context.
type StateContext struct {
	State State
	Data  interface{}
	Error error
}

// ResourceLoader manages resource loading with lifecycle methods.
type ResourceLoader struct {
	context StateContext
}

// NewResourceLoader creates a new ResourceLoader.
func NewResourceLoader() *ResourceLoader {
	return &ResourceLoader{
		context: StateContext{State: StateIdle},
	}
}

// Init initializes the loader.
// @lifecycle 1
func (r *ResourceLoader) Init() {
	r.context = StateContext{State: StateIdle}
}

// Start begins loading resources.
// @lifecycle 2
func (r *ResourceLoader) Start() error {
	if r.context.State != StateIdle {
		return ErrInvalidState
	}
	r.context.State = StateLoading
	return nil
}

// Stop stops the loader.
// @lifecycle 3
func (r *ResourceLoader) Stop() {
	r.context.State = StateIdle
	r.context.Data = nil
}

// Destroy cleans up resources.
// @lifecycle 4
func (r *ResourceLoader) Destroy() {
	r.Stop()
}

// Load transitions from Idle to Loading.
func (r *ResourceLoader) Load() {
	r.context.State = StateLoading
}

// OnSuccess transitions from Loading to Loaded.
func (r *ResourceLoader) OnSuccess(data interface{}) {
	r.context.State = StateLoaded
	r.context.Data = data
}

// OnError transitions from Loading to Error.
func (r *ResourceLoader) OnError(err error) {
	r.context.State = StateError
	r.context.Error = err
}

// Retry transitions from Error to Idle.
func (r *ResourceLoader) Retry() {
	if r.context.State == StateError {
		r.context.State = StateIdle
		r.context.Error = nil
	}
}

var ErrInvalidState = errors.New("invalid state")

import "errors"
