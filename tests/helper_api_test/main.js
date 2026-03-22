// Test: High-level helper API
Engine.setupScene({
  background: Engine.color(0.15, 0.15, 0.2),
  camera: { position: Engine.vec3(0, 0, 4) }
});

var box = Engine.createPrimitive("box", {
  color: Engine.color(1, 0.3, 0.1),
  position: Engine.vec3(-1.5, 0, 0)
});
Engine.addToScene(box);

var sphere = Engine.createPrimitive("sphere", {
  color: Engine.color(0.2, 0.8, 0.3),
  position: Engine.vec3(1.5, 0, 0)
});
Engine.addToScene(sphere);

Engine.onProcess(function(delta) {
  box.rotate_y(delta * 2);
  sphere.rotate_x(delta * 1.5);
});

console.log("Helper API test loaded successfully");
