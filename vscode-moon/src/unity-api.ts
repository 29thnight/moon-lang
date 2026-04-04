// Unity API database for autocomplete.
// Hardcoded core types and their members.

export interface UnityType {
    name: string;
    namespace: string;
    kind: 'class' | 'struct' | 'enum' | 'static';
    description: string;
    members: UnityMember[];
}

export interface UnityMember {
    name: string;
    kind: 'method' | 'property' | 'field' | 'event';
    type: string;
    description: string;
    params?: string; // e.g. "(float x, float y, float z)"
}

export const UNITY_API: UnityType[] = [
    // ── MonoBehaviour ────────────────────────────
    {
        name: 'MonoBehaviour', namespace: 'UnityEngine', kind: 'class',
        description: 'Base class for Unity scripts',
        members: [
            { name: 'gameObject', kind: 'property', type: 'GameObject', description: 'The game object this component is attached to' },
            { name: 'transform', kind: 'property', type: 'Transform', description: 'The Transform attached to this GameObject' },
            { name: 'enabled', kind: 'property', type: 'bool', description: 'Enabled state of the component' },
            { name: 'tag', kind: 'property', type: 'String', description: 'The tag of this game object' },
            { name: 'name', kind: 'property', type: 'String', description: 'The name of the object' },
            { name: 'StartCoroutine', kind: 'method', type: 'Coroutine', description: 'Starts a coroutine', params: '(IEnumerator routine)' },
            { name: 'StopCoroutine', kind: 'method', type: 'void', description: 'Stops a coroutine', params: '(Coroutine routine)' },
            { name: 'StopAllCoroutines', kind: 'method', type: 'void', description: 'Stops all coroutines', params: '()' },
            { name: 'Invoke', kind: 'method', type: 'void', description: 'Invokes a method after delay', params: '(string method, float time)' },
            { name: 'CancelInvoke', kind: 'method', type: 'void', description: 'Cancels all Invoke calls', params: '()' },
            { name: 'GetComponent', kind: 'method', type: 'T', description: 'Gets component of type T', params: '<T>()' },
            { name: 'GetComponentInChildren', kind: 'method', type: 'T', description: 'Gets component in children', params: '<T>()' },
            { name: 'GetComponentInParent', kind: 'method', type: 'T', description: 'Gets component in parent', params: '<T>()' },
            { name: 'Destroy', kind: 'method', type: 'void', description: 'Destroys a game object', params: '(Object obj, float t = 0)' },
            { name: 'Instantiate', kind: 'method', type: 'Object', description: 'Clones an object', params: '(Object original)' },
            { name: 'print', kind: 'method', type: 'void', description: 'Logs message to console', params: '(object message)' },
        ]
    },
    // ── GameObject ────────────────────────────────
    {
        name: 'GameObject', namespace: 'UnityEngine', kind: 'class',
        description: 'Base class for all entities in Unity scenes',
        members: [
            { name: 'transform', kind: 'property', type: 'Transform', description: 'The Transform of this object' },
            { name: 'activeSelf', kind: 'property', type: 'bool', description: 'Is this object active?' },
            { name: 'tag', kind: 'property', type: 'String', description: 'The tag of this object' },
            { name: 'name', kind: 'property', type: 'String', description: 'The name of this object' },
            { name: 'layer', kind: 'property', type: 'int', description: 'The layer of this object' },
            { name: 'SetActive', kind: 'method', type: 'void', description: 'Activates/deactivates the object', params: '(bool value)' },
            { name: 'GetComponent', kind: 'method', type: 'T', description: 'Gets component', params: '<T>()' },
            { name: 'AddComponent', kind: 'method', type: 'T', description: 'Adds component', params: '<T>()' },
            { name: 'CompareTag', kind: 'method', type: 'bool', description: 'Compares tag', params: '(string tag)' },
            { name: 'FindWithTag', kind: 'method', type: 'GameObject', description: 'Finds object by tag', params: '(string tag)' },
        ]
    },
    // ── Transform ─────────────────────────────────
    {
        name: 'Transform', namespace: 'UnityEngine', kind: 'class',
        description: 'Position, rotation and scale of an object',
        members: [
            { name: 'position', kind: 'property', type: 'Vector3', description: 'World space position' },
            { name: 'localPosition', kind: 'property', type: 'Vector3', description: 'Local space position' },
            { name: 'rotation', kind: 'property', type: 'Quaternion', description: 'World space rotation' },
            { name: 'localRotation', kind: 'property', type: 'Quaternion', description: 'Local space rotation' },
            { name: 'localScale', kind: 'property', type: 'Vector3', description: 'Local scale' },
            { name: 'eulerAngles', kind: 'property', type: 'Vector3', description: 'Rotation as Euler angles' },
            { name: 'forward', kind: 'property', type: 'Vector3', description: 'Forward direction (z)' },
            { name: 'right', kind: 'property', type: 'Vector3', description: 'Right direction (x)' },
            { name: 'up', kind: 'property', type: 'Vector3', description: 'Up direction (y)' },
            { name: 'parent', kind: 'property', type: 'Transform', description: 'Parent transform' },
            { name: 'childCount', kind: 'property', type: 'int', description: 'Number of children' },
            { name: 'Translate', kind: 'method', type: 'void', description: 'Moves the transform', params: '(Vector3 translation)' },
            { name: 'Rotate', kind: 'method', type: 'void', description: 'Rotates the transform', params: '(Vector3 eulers)' },
            { name: 'LookAt', kind: 'method', type: 'void', description: 'Points forward at target', params: '(Transform target)' },
            { name: 'GetChild', kind: 'method', type: 'Transform', description: 'Gets child by index', params: '(int index)' },
            { name: 'Find', kind: 'method', type: 'Transform', description: 'Finds child by name', params: '(string name)' },
        ]
    },
    // ── Rigidbody ─────────────────────────────────
    {
        name: 'Rigidbody', namespace: 'UnityEngine', kind: 'class',
        description: 'Physics body for 3D objects',
        members: [
            { name: 'velocity', kind: 'property', type: 'Vector3', description: 'Linear velocity' },
            { name: 'angularVelocity', kind: 'property', type: 'Vector3', description: 'Angular velocity' },
            { name: 'mass', kind: 'property', type: 'float', description: 'Mass of the body' },
            { name: 'drag', kind: 'property', type: 'float', description: 'Drag coefficient' },
            { name: 'useGravity', kind: 'property', type: 'bool', description: 'Use gravity?' },
            { name: 'isKinematic', kind: 'property', type: 'bool', description: 'Is kinematic?' },
            { name: 'position', kind: 'property', type: 'Vector3', description: 'Position of the rigidbody' },
            { name: 'AddForce', kind: 'method', type: 'void', description: 'Adds force to the body', params: '(Vector3 force)' },
            { name: 'AddTorque', kind: 'method', type: 'void', description: 'Adds torque', params: '(Vector3 torque)' },
            { name: 'MovePosition', kind: 'method', type: 'void', description: 'Moves to position', params: '(Vector3 position)' },
            { name: 'MoveRotation', kind: 'method', type: 'void', description: 'Rotates to rotation', params: '(Quaternion rot)' },
        ]
    },
    // ── Animator ───────────────────────────────────
    {
        name: 'Animator', namespace: 'UnityEngine', kind: 'class',
        description: 'Controls animations',
        members: [
            { name: 'speed', kind: 'property', type: 'float', description: 'Playback speed' },
            { name: 'Play', kind: 'method', type: 'void', description: 'Plays an animation state', params: '(string stateName)' },
            { name: 'SetBool', kind: 'method', type: 'void', description: 'Sets a bool parameter', params: '(string name, bool value)' },
            { name: 'SetFloat', kind: 'method', type: 'void', description: 'Sets a float parameter', params: '(string name, float value)' },
            { name: 'SetInteger', kind: 'method', type: 'void', description: 'Sets an int parameter', params: '(string name, int value)' },
            { name: 'SetTrigger', kind: 'method', type: 'void', description: 'Sets a trigger parameter', params: '(string name)' },
            { name: 'GetBool', kind: 'method', type: 'bool', description: 'Gets a bool parameter', params: '(string name)' },
            { name: 'GetFloat', kind: 'method', type: 'float', description: 'Gets a float parameter', params: '(string name)' },
        ]
    },
    // ── Collider ───────────────────────────────────
    {
        name: 'Collider', namespace: 'UnityEngine', kind: 'class',
        description: 'Base class for colliders',
        members: [
            { name: 'enabled', kind: 'property', type: 'bool', description: 'Enabled state' },
            { name: 'isTrigger', kind: 'property', type: 'bool', description: 'Is trigger?' },
            { name: 'bounds', kind: 'property', type: 'Bounds', description: 'Bounding volume' },
            { name: 'gameObject', kind: 'property', type: 'GameObject', description: 'The attached GameObject' },
            { name: 'transform', kind: 'property', type: 'Transform', description: 'The attached Transform' },
            { name: 'CompareTag', kind: 'method', type: 'bool', description: 'Compares tag', params: '(string tag)' },
            { name: 'ClosestPoint', kind: 'method', type: 'Vector3', description: 'Closest point on collider', params: '(Vector3 position)' },
        ]
    },
    // ── Collision ──────────────────────────────────
    {
        name: 'Collision', namespace: 'UnityEngine', kind: 'class',
        description: 'Collision event data',
        members: [
            { name: 'gameObject', kind: 'property', type: 'GameObject', description: 'The other GameObject' },
            { name: 'transform', kind: 'property', type: 'Transform', description: 'The other Transform' },
            { name: 'relativeVelocity', kind: 'property', type: 'Vector3', description: 'Relative velocity of collision' },
            { name: 'contactCount', kind: 'property', type: 'int', description: 'Number of contact points' },
            { name: 'collider', kind: 'property', type: 'Collider', description: 'The other Collider' },
        ]
    },
    // ── AudioSource ───────────────────────────────
    {
        name: 'AudioSource', namespace: 'UnityEngine', kind: 'class',
        description: 'Plays audio clips',
        members: [
            { name: 'clip', kind: 'property', type: 'AudioClip', description: 'The audio clip' },
            { name: 'volume', kind: 'property', type: 'float', description: 'Volume (0-1)' },
            { name: 'pitch', kind: 'property', type: 'float', description: 'Pitch multiplier' },
            { name: 'loop', kind: 'property', type: 'bool', description: 'Loop playback?' },
            { name: 'isPlaying', kind: 'property', type: 'bool', description: 'Is currently playing?' },
            { name: 'Play', kind: 'method', type: 'void', description: 'Plays the clip', params: '()' },
            { name: 'Stop', kind: 'method', type: 'void', description: 'Stops playback', params: '()' },
            { name: 'Pause', kind: 'method', type: 'void', description: 'Pauses playback', params: '()' },
            { name: 'PlayOneShot', kind: 'method', type: 'void', description: 'Plays clip once', params: '(AudioClip clip)' },
        ]
    },
    // ── SpriteRenderer ────────────────────────────
    {
        name: 'SpriteRenderer', namespace: 'UnityEngine', kind: 'class',
        description: 'Renders sprites',
        members: [
            { name: 'sprite', kind: 'property', type: 'Sprite', description: 'The sprite to render' },
            { name: 'color', kind: 'property', type: 'Color', description: 'Rendering color' },
            { name: 'flipX', kind: 'property', type: 'bool', description: 'Flip horizontally' },
            { name: 'flipY', kind: 'property', type: 'bool', description: 'Flip vertically' },
            { name: 'enabled', kind: 'property', type: 'bool', description: 'Enabled state' },
        ]
    },
    // ── Camera ────────────────────────────────────
    {
        name: 'Camera', namespace: 'UnityEngine', kind: 'class',
        description: 'Camera component',
        members: [
            { name: 'main', kind: 'property', type: 'Camera', description: 'The main camera' },
            { name: 'fieldOfView', kind: 'property', type: 'float', description: 'Field of view' },
            { name: 'orthographic', kind: 'property', type: 'bool', description: 'Is orthographic?' },
            { name: 'orthographicSize', kind: 'property', type: 'float', description: 'Orthographic size' },
            { name: 'ScreenToWorldPoint', kind: 'method', type: 'Vector3', description: 'Screen to world', params: '(Vector3 position)' },
            { name: 'WorldToScreenPoint', kind: 'method', type: 'Vector3', description: 'World to screen', params: '(Vector3 position)' },
        ]
    },
    // ── Static classes ────────────────────────────
    {
        name: 'Input', namespace: 'UnityEngine', kind: 'static',
        description: 'Input system',
        members: [
            { name: 'GetAxis', kind: 'method', type: 'float', description: 'Gets axis value', params: '(string axisName)' },
            { name: 'GetKey', kind: 'method', type: 'bool', description: 'Is key held?', params: '(KeyCode key)' },
            { name: 'GetKeyDown', kind: 'method', type: 'bool', description: 'Key pressed this frame?', params: '(KeyCode key)' },
            { name: 'GetKeyUp', kind: 'method', type: 'bool', description: 'Key released this frame?', params: '(KeyCode key)' },
            { name: 'GetButton', kind: 'method', type: 'bool', description: 'Is button held?', params: '(string buttonName)' },
            { name: 'GetButtonDown', kind: 'method', type: 'bool', description: 'Button pressed?', params: '(string buttonName)' },
            { name: 'GetMouseButton', kind: 'method', type: 'bool', description: 'Mouse button held?', params: '(int button)' },
            { name: 'mousePosition', kind: 'property', type: 'Vector3', description: 'Mouse pixel position' },
        ]
    },
    {
        name: 'Time', namespace: 'UnityEngine', kind: 'static',
        description: 'Time management',
        members: [
            { name: 'deltaTime', kind: 'property', type: 'float', description: 'Time since last frame' },
            { name: 'fixedDeltaTime', kind: 'property', type: 'float', description: 'Fixed timestep interval' },
            { name: 'time', kind: 'property', type: 'float', description: 'Time since startup' },
            { name: 'timeScale', kind: 'property', type: 'float', description: 'Time scale factor' },
            { name: 'unscaledDeltaTime', kind: 'property', type: 'float', description: 'Unscaled delta time' },
            { name: 'frameCount', kind: 'property', type: 'int', description: 'Total frames rendered' },
        ]
    },
    {
        name: 'Debug', namespace: 'UnityEngine', kind: 'static',
        description: 'Debug utilities',
        members: [
            { name: 'Log', kind: 'method', type: 'void', description: 'Logs message', params: '(object message)' },
            { name: 'LogWarning', kind: 'method', type: 'void', description: 'Logs warning', params: '(object message)' },
            { name: 'LogError', kind: 'method', type: 'void', description: 'Logs error', params: '(object message)' },
            { name: 'DrawRay', kind: 'method', type: 'void', description: 'Draws a debug ray', params: '(Vector3 start, Vector3 dir)' },
            { name: 'DrawLine', kind: 'method', type: 'void', description: 'Draws a debug line', params: '(Vector3 start, Vector3 end)' },
        ]
    },
    {
        name: 'Physics', namespace: 'UnityEngine', kind: 'static',
        description: '3D physics queries',
        members: [
            { name: 'Raycast', kind: 'method', type: 'bool', description: 'Casts a ray', params: '(Vector3 origin, Vector3 direction, float maxDistance)' },
            { name: 'OverlapSphere', kind: 'method', type: 'Collider[]', description: 'Gets colliders in sphere', params: '(Vector3 position, float radius)' },
            { name: 'gravity', kind: 'property', type: 'Vector3', description: 'Global gravity' },
        ]
    },
    {
        name: 'Mathf', namespace: 'UnityEngine', kind: 'static',
        description: 'Math utilities',
        members: [
            { name: 'Abs', kind: 'method', type: 'float', description: 'Absolute value', params: '(float f)' },
            { name: 'Clamp', kind: 'method', type: 'float', description: 'Clamps value', params: '(float value, float min, float max)' },
            { name: 'Lerp', kind: 'method', type: 'float', description: 'Linear interpolation', params: '(float a, float b, float t)' },
            { name: 'Min', kind: 'method', type: 'float', description: 'Minimum value', params: '(float a, float b)' },
            { name: 'Max', kind: 'method', type: 'float', description: 'Maximum value', params: '(float a, float b)' },
            { name: 'Sin', kind: 'method', type: 'float', description: 'Sine', params: '(float f)' },
            { name: 'Cos', kind: 'method', type: 'float', description: 'Cosine', params: '(float f)' },
            { name: 'Sqrt', kind: 'method', type: 'float', description: 'Square root', params: '(float f)' },
            { name: 'PI', kind: 'field', type: 'float', description: '3.14159...' },
            { name: 'Infinity', kind: 'field', type: 'float', description: 'Positive infinity' },
        ]
    },
    // ── Vector3 ───────────────────────────────────
    {
        name: 'Vector3', namespace: 'UnityEngine', kind: 'struct',
        description: '3D vector',
        members: [
            { name: 'x', kind: 'field', type: 'float', description: 'X component' },
            { name: 'y', kind: 'field', type: 'float', description: 'Y component' },
            { name: 'z', kind: 'field', type: 'float', description: 'Z component' },
            { name: 'magnitude', kind: 'property', type: 'float', description: 'Length of vector' },
            { name: 'normalized', kind: 'property', type: 'Vector3', description: 'Unit vector' },
            { name: 'sqrMagnitude', kind: 'property', type: 'float', description: 'Squared length' },
            { name: 'zero', kind: 'property', type: 'Vector3', description: '(0,0,0)' },
            { name: 'one', kind: 'property', type: 'Vector3', description: '(1,1,1)' },
            { name: 'forward', kind: 'property', type: 'Vector3', description: '(0,0,1)' },
            { name: 'up', kind: 'property', type: 'Vector3', description: '(0,1,0)' },
            { name: 'right', kind: 'property', type: 'Vector3', description: '(1,0,0)' },
            { name: 'Distance', kind: 'method', type: 'float', description: 'Distance between two points', params: '(Vector3 a, Vector3 b)' },
            { name: 'Lerp', kind: 'method', type: 'Vector3', description: 'Linear interpolation', params: '(Vector3 a, Vector3 b, float t)' },
            { name: 'Dot', kind: 'method', type: 'float', description: 'Dot product', params: '(Vector3 a, Vector3 b)' },
            { name: 'Cross', kind: 'method', type: 'Vector3', description: 'Cross product', params: '(Vector3 a, Vector3 b)' },
            { name: 'Normalize', kind: 'method', type: 'void', description: 'Normalizes this vector', params: '()' },
        ]
    },
    // ── SceneManager ──────────────────────────────
    {
        name: 'SceneManager', namespace: 'UnityEngine.SceneManagement', kind: 'static',
        description: 'Scene loading and management',
        members: [
            { name: 'LoadScene', kind: 'method', type: 'void', description: 'Loads a scene', params: '(string sceneName)' },
            { name: 'LoadSceneAsync', kind: 'method', type: 'AsyncOperation', description: 'Loads scene async', params: '(string sceneName)' },
            { name: 'GetActiveScene', kind: 'method', type: 'Scene', description: 'Gets the active scene', params: '()' },
        ]
    },
    // ── Application ───────────────────────────────
    {
        name: 'Application', namespace: 'UnityEngine', kind: 'static',
        description: 'Application info and control',
        members: [
            { name: 'Quit', kind: 'method', type: 'void', description: 'Quits the application', params: '()' },
            { name: 'targetFrameRate', kind: 'property', type: 'int', description: 'Target frame rate' },
            { name: 'platform', kind: 'property', type: 'RuntimePlatform', description: 'Current platform' },
            { name: 'isPlaying', kind: 'property', type: 'bool', description: 'Is playing in editor?' },
        ]
    },
    // ── UI: Button ────────────────────────────────
    {
        name: 'Button', namespace: 'UnityEngine.UI', kind: 'class',
        description: 'UI Button',
        members: [
            { name: 'onClick', kind: 'event', type: 'UnityEvent', description: 'Click event' },
            { name: 'interactable', kind: 'property', type: 'bool', description: 'Is interactable?' },
        ]
    },
    {
        name: 'Slider', namespace: 'UnityEngine.UI', kind: 'class',
        description: 'UI Slider',
        members: [
            { name: 'value', kind: 'property', type: 'float', description: 'Current value' },
            { name: 'minValue', kind: 'property', type: 'float', description: 'Minimum value' },
            { name: 'maxValue', kind: 'property', type: 'float', description: 'Maximum value' },
            { name: 'onValueChanged', kind: 'event', type: 'UnityEvent<float>', description: 'Value changed event' },
        ]
    },
];

// Namespace → type names mapping
export const NAMESPACE_TYPES: Record<string, string[]> = {};
for (const t of UNITY_API) {
    const ns = t.namespace;
    if (!NAMESPACE_TYPES[ns]) { NAMESPACE_TYPES[ns] = []; }
    NAMESPACE_TYPES[ns].push(t.name);
}

// ── Primitive Types (not in Unity DB) ────────────

const PRIMITIVE_TYPES: UnityType[] = [
    {
        name: 'Int', namespace: '', kind: 'struct',
        description: 'Moon Int (System.Int32)',
        members: [
            { name: 'toString', kind: 'method', type: 'String', description: 'Convert to string', params: '()' },
            { name: 'toFloat', kind: 'method', type: 'Float', description: 'Convert to Float', params: '()' },
            { name: 'toDouble', kind: 'method', type: 'Double', description: 'Convert to Double', params: '()' },
            { name: 'toLong', kind: 'method', type: 'Long', description: 'Convert to Long', params: '()' },
            { name: 'compareTo', kind: 'method', type: 'Int', description: 'Compare to another Int', params: '(Int other)' },
            { name: 'equals', kind: 'method', type: 'Bool', description: 'Value equality', params: '(Int other)' },
            { name: 'abs', kind: 'method', type: 'Int', description: 'Absolute value', params: '()' },
            { name: 'clamp', kind: 'method', type: 'Int', description: 'Clamp between min and max', params: '(Int min, Int max)' },
            { name: 'MaxValue', kind: 'field', type: 'Int', description: '2147483647' },
            { name: 'MinValue', kind: 'field', type: 'Int', description: '-2147483648' },
            { name: 'Parse', kind: 'method', type: 'Int', description: 'Parse from string', params: '(String s)' },
            { name: 'TryParse', kind: 'method', type: 'Bool', description: 'Try parse from string', params: '(String s, out Int result)' },
        ]
    },
    {
        name: 'Float', namespace: '', kind: 'struct',
        description: 'Moon Float (System.Single)',
        members: [
            { name: 'toString', kind: 'method', type: 'String', description: 'Convert to string', params: '()' },
            { name: 'toInt', kind: 'method', type: 'Int', description: 'Truncate to Int', params: '()' },
            { name: 'toDouble', kind: 'method', type: 'Double', description: 'Convert to Double', params: '()' },
            { name: 'compareTo', kind: 'method', type: 'Int', description: 'Compare to another Float', params: '(Float other)' },
            { name: 'equals', kind: 'method', type: 'Bool', description: 'Value equality', params: '(Float other)' },
            { name: 'isNaN', kind: 'method', type: 'Bool', description: 'Is Not-a-Number', params: '()' },
            { name: 'isInfinity', kind: 'method', type: 'Bool', description: 'Is infinity', params: '()' },
            { name: 'MaxValue', kind: 'field', type: 'Float', description: '3.4028235E+38' },
            { name: 'MinValue', kind: 'field', type: 'Float', description: '-3.4028235E+38' },
            { name: 'Epsilon', kind: 'field', type: 'Float', description: 'Smallest positive Float' },
            { name: 'NaN', kind: 'field', type: 'Float', description: 'Not a Number' },
            { name: 'PositiveInfinity', kind: 'field', type: 'Float', description: 'Positive infinity' },
            { name: 'NegativeInfinity', kind: 'field', type: 'Float', description: 'Negative infinity' },
            { name: 'Parse', kind: 'method', type: 'Float', description: 'Parse from string', params: '(String s)' },
        ]
    },
    {
        name: 'Double', namespace: '', kind: 'struct',
        description: 'Moon Double (System.Double)',
        members: [
            { name: 'toString', kind: 'method', type: 'String', description: 'Convert to string', params: '()' },
            { name: 'toInt', kind: 'method', type: 'Int', description: 'Truncate to Int', params: '()' },
            { name: 'toFloat', kind: 'method', type: 'Float', description: 'Convert to Float', params: '()' },
            { name: 'compareTo', kind: 'method', type: 'Int', description: 'Compare', params: '(Double other)' },
            { name: 'equals', kind: 'method', type: 'Bool', description: 'Value equality', params: '(Double other)' },
            { name: 'isNaN', kind: 'method', type: 'Bool', description: 'Is Not-a-Number', params: '()' },
            { name: 'MaxValue', kind: 'field', type: 'Double', description: 'Max value' },
            { name: 'MinValue', kind: 'field', type: 'Double', description: 'Min value' },
            { name: 'Parse', kind: 'method', type: 'Double', description: 'Parse from string', params: '(String s)' },
        ]
    },
    {
        name: 'Bool', namespace: '', kind: 'struct',
        description: 'Moon Bool (System.Boolean)',
        members: [
            { name: 'toString', kind: 'method', type: 'String', description: 'Convert to string', params: '()' },
            { name: 'equals', kind: 'method', type: 'Bool', description: 'Value equality', params: '(Bool other)' },
            { name: 'compareTo', kind: 'method', type: 'Int', description: 'Compare', params: '(Bool other)' },
            { name: 'Parse', kind: 'method', type: 'Bool', description: 'Parse from string', params: '(String s)' },
            { name: 'TrueString', kind: 'field', type: 'String', description: '"True"' },
            { name: 'FalseString', kind: 'field', type: 'String', description: '"False"' },
        ]
    },
    {
        name: 'String', namespace: '', kind: 'class',
        description: 'Moon String (System.String)',
        members: [
            { name: 'length', kind: 'property', type: 'Int', description: 'Character count' },
            { name: 'isEmpty', kind: 'method', type: 'Bool', description: 'Is empty string', params: '()' },
            { name: 'contains', kind: 'method', type: 'Bool', description: 'Contains substring', params: '(String value)' },
            { name: 'startsWith', kind: 'method', type: 'Bool', description: 'Starts with prefix', params: '(String value)' },
            { name: 'endsWith', kind: 'method', type: 'Bool', description: 'Ends with suffix', params: '(String value)' },
            { name: 'indexOf', kind: 'method', type: 'Int', description: 'Index of substring', params: '(String value)' },
            { name: 'lastIndexOf', kind: 'method', type: 'Int', description: 'Last index of substring', params: '(String value)' },
            { name: 'substring', kind: 'method', type: 'String', description: 'Extract substring', params: '(Int startIndex, Int length)' },
            { name: 'replace', kind: 'method', type: 'String', description: 'Replace occurrences', params: '(String oldValue, String newValue)' },
            { name: 'trim', kind: 'method', type: 'String', description: 'Trim whitespace', params: '()' },
            { name: 'trimStart', kind: 'method', type: 'String', description: 'Trim leading whitespace', params: '()' },
            { name: 'trimEnd', kind: 'method', type: 'String', description: 'Trim trailing whitespace', params: '()' },
            { name: 'toUpper', kind: 'method', type: 'String', description: 'To uppercase', params: '()' },
            { name: 'toLower', kind: 'method', type: 'String', description: 'To lowercase', params: '()' },
            { name: 'split', kind: 'method', type: 'Array<String>', description: 'Split by separator', params: '(String separator)' },
            { name: 'toInt', kind: 'method', type: 'Int', description: 'Parse as Int', params: '()' },
            { name: 'toFloat', kind: 'method', type: 'Float', description: 'Parse as Float', params: '()' },
            { name: 'equals', kind: 'method', type: 'Bool', description: 'Value equality', params: '(String other)' },
            { name: 'compareTo', kind: 'method', type: 'Int', description: 'Compare', params: '(String other)' },
            { name: 'toString', kind: 'method', type: 'String', description: 'Returns self', params: '()' },
            { name: 'Empty', kind: 'field', type: 'String', description: 'Empty string ""' },
            { name: 'IsNullOrEmpty', kind: 'method', type: 'Bool', description: 'Is null or empty', params: '(String value)' },
            { name: 'IsNullOrWhiteSpace', kind: 'method', type: 'Bool', description: 'Is null or whitespace', params: '(String value)' },
            { name: 'Format', kind: 'method', type: 'String', description: 'Format string', params: '(String format, ...)' },
            { name: 'Join', kind: 'method', type: 'String', description: 'Join strings', params: '(String separator, Array<String> values)' },
        ]
    },
    {
        name: 'Char', namespace: '', kind: 'struct',
        description: 'Moon Char (System.Char)',
        members: [
            { name: 'toString', kind: 'method', type: 'String', description: 'Convert to string', params: '()' },
            { name: 'isDigit', kind: 'method', type: 'Bool', description: 'Is digit character', params: '()' },
            { name: 'isLetter', kind: 'method', type: 'Bool', description: 'Is letter character', params: '()' },
            { name: 'isWhiteSpace', kind: 'method', type: 'Bool', description: 'Is whitespace', params: '()' },
            { name: 'isUpper', kind: 'method', type: 'Bool', description: 'Is uppercase', params: '()' },
            { name: 'isLower', kind: 'method', type: 'Bool', description: 'Is lowercase', params: '()' },
            { name: 'toUpper', kind: 'method', type: 'Char', description: 'To uppercase', params: '()' },
            { name: 'toLower', kind: 'method', type: 'Char', description: 'To lowercase', params: '()' },
        ]
    },
    {
        name: 'Long', namespace: '', kind: 'struct',
        description: 'Moon Long (System.Int64)',
        members: [
            { name: 'toString', kind: 'method', type: 'String', description: 'Convert to string', params: '()' },
            { name: 'toInt', kind: 'method', type: 'Int', description: 'Convert to Int (may overflow)', params: '()' },
            { name: 'toFloat', kind: 'method', type: 'Float', description: 'Convert to Float', params: '()' },
            { name: 'toDouble', kind: 'method', type: 'Double', description: 'Convert to Double', params: '()' },
            { name: 'equals', kind: 'method', type: 'Bool', description: 'Value equality', params: '(Long other)' },
            { name: 'MaxValue', kind: 'field', type: 'Long', description: '9223372036854775807' },
            { name: 'MinValue', kind: 'field', type: 'Long', description: '-9223372036854775808' },
            { name: 'Parse', kind: 'method', type: 'Long', description: 'Parse from string', params: '(String s)' },
        ]
    },
    {
        name: 'Byte', namespace: '', kind: 'struct',
        description: 'Moon Byte (System.Byte)',
        members: [
            { name: 'toString', kind: 'method', type: 'String', description: 'Convert to string', params: '()' },
            { name: 'toInt', kind: 'method', type: 'Int', description: 'Convert to Int', params: '()' },
            { name: 'equals', kind: 'method', type: 'Bool', description: 'Value equality', params: '(Byte other)' },
            { name: 'MaxValue', kind: 'field', type: 'Byte', description: '255' },
            { name: 'MinValue', kind: 'field', type: 'Byte', description: '0' },
            { name: 'Parse', kind: 'method', type: 'Byte', description: 'Parse from string', params: '(String s)' },
        ]
    },
];

// Quick lookup by type name
export const TYPE_MAP: Record<string, UnityType> = {};
for (const t of UNITY_API) {
    TYPE_MAP[t.name] = t;
}
for (const t of PRIMITIVE_TYPES) {
    TYPE_MAP[t.name] = t;
}

// Moon keywords for completion
export const MOON_KEYWORDS = [
    'component', 'asset', 'class', 'data', 'enum',
    'serialize', 'require', 'optional', 'child', 'parent',
    'val', 'var', 'func', 'coroutine', 'override', 'return',
    'if', 'else', 'when', 'for', 'while', 'in', 'until', 'break', 'continue',
    'wait', 'start', 'stop', 'stopAll', 'listen', 'intrinsic',
    'using', 'null', 'this', 'true', 'false',
    'awake', 'update', 'fixedUpdate', 'lateUpdate',
    'onEnable', 'onDisable', 'onDestroy',
    'onTriggerEnter', 'onTriggerExit', 'onCollisionEnter', 'onCollisionExit',
    'nextFrame', 'fixedFrame',
    'public', 'private', 'protected',
];

// Moon sugar functions
export interface MoonBuiltin {
    name: string;
    params: string;
    description: string;
    moonOnly?: boolean;
}

export const MOON_BUILTINS: MoonBuiltin[] = [
    { name: 'vec2', params: '(x, y)', description: 'Create Vector2', moonOnly: true },
    { name: 'vec3', params: '(x, y, z)', description: 'Create Vector3', moonOnly: true },
    { name: 'color', params: '(r, g, b, a)', description: 'Create Color', moonOnly: true },
    { name: 'get', params: '<T>()', description: 'GetComponent<T>()', moonOnly: true },
    { name: 'find', params: '<T>()', description: 'FindFirstObjectByType<T>()', moonOnly: true },
    { name: 'Destroy', params: '(obj)', description: 'Destroy object' },
    { name: 'print', params: '(message, level?)', description: 'Debug.Log / LogWarning / LogError. Level: Log (default), Warn, Error', moonOnly: true },
    { name: 'log', params: '(message)', description: 'Debug.Log(message)', moonOnly: true },
    { name: 'warn', params: '(message)', description: 'Debug.LogWarning(message)', moonOnly: true },
    { name: 'error', params: '(message)', description: 'Debug.LogError(message)', moonOnly: true },
];
